//! Stash service: hide windows at screen edges, reveal on hover.
//!
//! Ports `WindowStashService`: edge choice by nearest work-area edge,
//! peek-strip frames, hover hit-zones with reveal-delay arming, FIFO
//! reveal-next, DPI/work-area rebasing, unambiguous startup restore, and
//! persistence of only fully-identified records. There are no strip
//! windows — hover is purely geometric.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use windows::Win32::UI::WindowsAndMessaging::*;

use super::native;
use crate::core::rect::{Point, Rect};
use crate::core::stash::{
    calculate_stashed_frame, nearest_edge, StashEdge, StashMonitor, StashPlacement, StashPoint,
    StashRecord, StashRect,
};
use crate::core::stash_rebase::{find_restore_monitor, rebase_rect};

const POLL_INTERVAL_MS: u64 = 80;

struct LiveRecord {
    edge: StashEdge,
    placement: WINDOWPLACEMENT,
    monitor: StashMonitor,
    identity: crate::core::identity::WindowIdentity,
    persisted_id: String,
}

struct StashLive {
    by_hwnd: HashMap<u64, LiveRecord>,
    order: Vec<u64>,
    pending: Option<(u64, u64)>,
    last_poll: u64,
}

static LIVE: OnceLock<Mutex<StashLive>> = OnceLock::new();

fn live() -> &'static Mutex<StashLive> {
    LIVE.get_or_init(|| {
        Mutex::new(StashLive {
            by_hwnd: HashMap::new(),
            order: Vec::new(),
            pending: None,
            last_poll: 0,
        })
    })
}

/// Stash a window at its nearest work-area edge.
pub fn stash(hwnd: u64) -> Result<String, String> {
    use crate::core::actions::WindowAction;
    if hwnd == 0 || !native::is_window(native::from_raw(hwnd as isize)) {
        return Err("The target window is no longer available.".to_string());
    }
    if let Err(diagnostic) = super::policy::try_authorize_action(hwnd, WindowAction::Stash) {
        return Err(diagnostic.to_string());
    }
    {
        let guard = live()
            .lock()
            .map_err(|_| "Internal state is unavailable.".to_string())?;
        if guard.by_hwnd.contains_key(&hwnd) {
            return Err("The target window is already stashed.".to_string());
        }
    }
    let native_hwnd = native::from_raw(hwnd as isize);
    let identity = native::window_identity(native_hwnd)
        .ok_or_else(|| "Could not identify the target window.".to_string())?;
    let original = native::window_placement(native_hwnd)
        .ok_or_else(|| "Could not identify the target window.".to_string())?;
    let _ = native::window_rect(native_hwnd)
        .ok_or_else(|| "Could not identify the target window.".to_string())?;
    // Collapse maximized/minimized so geometry is measurable.
    native::show_window(native_hwnd, SW_RESTORE);
    let current = native::window_rect(native_hwnd)
        .ok_or_else(|| "Could not identify the target window.".to_string())?;
    let snapshot = super::monitor_service::for_rect(current)
        .ok_or_else(|| "Could not determine the target monitor.".to_string())?;
    let settings = super::shared::snapshot();
    let edge = nearest_edge(snapshot.work, current);
    let frame = calculate_stashed_frame(snapshot.work, current, edge, settings.stash_peek);
    if !native::set_pos(native_hwnd, frame) {
        let _ = native::set_placement(native_hwnd, &original);
        return Err(
            "Windows rejected the stash. The target may be elevated, protected, or non-movable."
                .to_string(),
        );
    }
    let can_persist = settings.stash_persistence_enabled
        && !identity.executable_path.is_empty()
        && !identity.window_class.is_empty();
    let persisted_id = if can_persist {
        uuid::Uuid::new_v4().simple().to_string()
    } else {
        String::new()
    };
    let monitor = StashMonitor {
        monitor: StashRect::from(snapshot.monitor),
        work: StashRect::from(snapshot.work),
        dpi_x: snapshot.dpi_x,
        dpi_y: snapshot.dpi_y,
    };
    {
        let mut guard = live()
            .lock()
            .map_err(|_| "Internal state is unavailable.".to_string())?;
        guard.by_hwnd.insert(
            hwnd,
            LiveRecord {
                edge,
                placement: original,
                monitor,
                identity: identity.clone(),
                persisted_id: persisted_id.clone(),
            },
        );
        guard.order.retain(|id| *id != hwnd);
        guard.order.push(hwnd);
    }
    if can_persist {
        persist_upsert(&StashRecord {
            id: persisted_id,
            executable_path: identity.executable_path,
            process_id: identity.process_id,
            window_class: identity.window_class,
            title: identity.title,
            edge,
            original_placement: placement_to_stash(&original),
            original_monitor: live_monitor(hwnd),
            stashed_frame: StashRect::from(frame),
        });
    }
    Ok(format!(
        "Stashed the window at the {} edge",
        edge_name(edge)
    ))
}

/// Reveal the first stashed window in FIFO order (hotkey/command path).
pub fn reveal_next() -> Result<String, String> {
    prune_stale();
    let target = live()
        .lock()
        .ok()
        .and_then(|guard| guard.order.iter().copied().next());
    match target {
        Some(hwnd) => reveal(hwnd),
        None => Err("No stashed windows to reveal.".to_string()),
    }
}

/// 80 ms-throttled poll: prune dead entries, reveal at cursor.
pub fn poll(cursor: Point) -> Option<String> {
    let now = native::tick_count();
    let due = live()
        .lock()
        .map(|guard| now.saturating_sub(guard.last_poll) >= POLL_INTERVAL_MS)
        .unwrap_or(true);
    if !due {
        return None;
    }
    if let Ok(mut guard) = live().lock() {
        guard.last_poll = now;
    }
    prune_stale();
    reveal_at_cursor(cursor)
}

/// Reveal whichever stashed window's hit zone holds the cursor.
pub fn reveal_at_cursor(cursor: Point) -> Option<String> {
    let settings = super::shared::snapshot();
    let zone = settings.stash_hit_zone.clamp(1, 96);
    let delay = settings.stash_reveal_delay_ms.clamp(0, 2000) as u64;
    let candidates: Vec<(u64, StashEdge, Rect)> = {
        let guard = live().lock().ok()?;
        guard
            .order
            .iter()
            .filter_map(|hwnd| {
                let record = guard.by_hwnd.get(hwnd)?;
                let work: Rect = record.monitor.work.into();
                Some((*hwnd, record.edge, work))
            })
            .collect()
    };
    for (hwnd, edge, work) in candidates {
        if !in_hit_zone(cursor, edge, work, zone) {
            continue;
        }
        if delay == 0 {
            return reveal(hwnd).ok();
        }
        let now = native::tick_count();
        let mut guard = live().lock().ok()?;
        match guard.pending {
            Some((pending_hwnd, started)) if pending_hwnd == hwnd => {
                if now.saturating_sub(started) >= delay {
                    guard.pending = None;
                    drop(guard);
                    return reveal(hwnd).ok();
                }
                return None;
            }
            _ => {
                guard.pending = Some((hwnd, now));
                return None;
            }
        }
    }
    if let Ok(mut guard) = live().lock() {
        guard.pending = None;
    }
    None
}

fn reveal(hwnd: u64) -> Result<String, String> {
    let record = live()
        .lock()
        .ok()
        .and_then(|guard| guard.by_hwnd.get(&hwnd).map(clone_live))
        .ok_or_else(|| "The target window is not stashed.".to_string())?;
    if !super::policy::is_eligible_for_enumeration(hwnd as isize, 0) {
        return Err("The stashed window is excluded or no longer an eligible target.".to_string());
    }
    let monitors = super::monitor_service::all();
    let mut placement = record.placement;
    if let Some(target) = find_restore_monitor(&record.monitor, &monitors) {
        let target_monitor = StashMonitor {
            monitor: StashRect::from(target.monitor),
            work: StashRect::from(target.work),
            dpi_x: target.dpi_x,
            dpi_y: target.dpi_y,
        };
        let normal: Rect = placement_to_stash(&placement).normal_position.into();
        let rebased = rebase_rect(normal, &record.monitor, &target_monitor);
        placement.rcNormalPosition = native::rect_to_native(rebased);
    }
    if !native::set_placement(native::from_raw(hwnd as isize), &placement) {
        if !native::is_window(native::from_raw(hwnd as isize)) {
            remove_runtime(hwnd, true);
        }
        return Err(
            "Windows rejected the reveal. The target may be elevated or closed.".to_string(),
        );
    }
    remove_runtime(hwnd, true);
    Ok("Revealed a stashed window".to_string())
}

#[derive(Clone)]
struct LiveSnapshot {
    placement: WINDOWPLACEMENT,
    monitor: StashMonitor,
}

fn clone_live(record: &LiveRecord) -> LiveSnapshot {
    LiveSnapshot {
        placement: record.placement,
        monitor: record.monitor.clone(),
    }
}

/// Drop dead entries (runtime + persisted), saving iff anything changed.
pub fn prune_stale() {
    let dead: Vec<u64> = live()
        .lock()
        .map(|guard| {
            guard
                .order
                .iter()
                .copied()
                .filter(|hwnd| !is_stash_alive(*hwnd))
                .collect()
        })
        .unwrap_or_default();
    if dead.is_empty() {
        return;
    }
    for hwnd in dead {
        remove_runtime(hwnd, true);
    }
}

fn is_stash_alive(hwnd: u64) -> bool {
    let native_hwnd = native::from_raw(hwnd as isize);
    if hwnd == 0 || !native::is_window(native_hwnd) {
        return false;
    }
    let pid = native::process_id(native_hwnd);
    if pid == 0 {
        return false;
    }
    let recorded_pid = live()
        .lock()
        .ok()
        .and_then(|guard| guard.by_hwnd.get(&hwnd).map(|r| r.identity.process_id));
    if recorded_pid != Some(pid) {
        // PID reused by an unrelated process: verify class still matches.
        let class = native::window_class(native_hwnd);
        let same_class = live().lock().ok().map(|guard| {
            guard
                .by_hwnd
                .get(&hwnd)
                .map(|r| {
                    !r.identity.window_class.is_empty()
                        && r.identity.window_class.eq_ignore_ascii_case(&class)
                })
                .unwrap_or(false)
        });
        if same_class != Some(true) {
            return false;
        }
    }
    true
}

fn remove_runtime(hwnd: u64, persist: bool) {
    let persisted_id = live()
        .lock()
        .ok()
        .and_then(|mut guard| {
            guard.order.retain(|id| *id != hwnd);
            if guard.pending.map(|(pending, _)| pending) == Some(hwnd) {
                guard.pending = None;
            }
            guard.by_hwnd.remove(&hwnd).map(|r| r.persisted_id)
        })
        .unwrap_or_default();
    if persist && !persisted_id.is_empty() {
        persist_remove(&persisted_id);
    }
}

/// Restore every alive window (clean quit): nothing stays off-screen.
pub fn restore_all() {
    let hwnds: Vec<u64> = live()
        .lock()
        .map(|guard| guard.order.clone())
        .unwrap_or_default();
    for hwnd in hwnds {
        let _ = reveal_quiet(hwnd);
    }
    if let Ok(mut guard) = live().lock() {
        guard.pending = None;
    }
}

fn reveal_quiet(hwnd: u64) -> bool {
    let record = live()
        .lock()
        .ok()
        .and_then(|guard| guard.by_hwnd.get(&hwnd).map(clone_live));
    let Some(record) = record else {
        remove_runtime(hwnd, true);
        return false;
    };
    let monitors = super::monitor_service::all();
    let mut placement = record.placement;
    if let Some(target) = find_restore_monitor(&record.monitor, &monitors) {
        let target_monitor = StashMonitor {
            monitor: StashRect::from(target.monitor),
            work: StashRect::from(target.work),
            dpi_x: target.dpi_x,
            dpi_y: target.dpi_y,
        };
        let normal: Rect = placement_to_stash(&placement).normal_position.into();
        placement.rcNormalPosition =
            native::rect_to_native(rebase_rect(normal, &record.monitor, &target_monitor));
    }
    let ok = native::set_placement(native::from_raw(hwnd as isize), &placement);
    remove_runtime(hwnd, true);
    ok
}

/// Re-stash persisted records whose live window matches unambiguously.
pub fn restore_persisted() {
    use crate::core::identity::{find_unambiguous_match, WindowIdentity};
    if !super::shared::snapshot().stash_persistence_enabled {
        return;
    }
    let records = super::shared::snapshot().stash_records;
    if records.is_empty() {
        return;
    }
    let mut claimed: Vec<u64> = Vec::new();
    for record in &records {
        let candidates: Vec<(u64, WindowIdentity)> = super::query::enumerate(0)
            .into_iter()
            .filter(|candidate| !claimed.contains(&candidate.hwnd))
            .filter_map(|candidate| {
                let identity = native::window_identity(native::from_raw(candidate.hwnd as isize))?;
                Some((candidate.hwnd, identity))
            })
            .collect();
        let Some(hwnd) = find_unambiguous_match(record, &candidates) else {
            continue;
        };
        claimed.push(hwnd);
        restash_persisted(hwnd, record);
    }
}

fn restash_persisted(hwnd: u64, record: &StashRecord) {
    let native_hwnd = native::from_raw(hwnd as isize);
    let Some(previous) = native::window_placement(native_hwnd) else {
        return;
    };
    native::show_window(native_hwnd, SW_RESTORE);
    let (current, snapshot) = match native::window_rect(native_hwnd)
        .and_then(|rect| super::monitor_service::for_rect(rect).map(|s| (rect, s)))
    {
        Some(pair) => pair,
        None => {
            let _ = native::set_placement(native_hwnd, &previous);
            return;
        }
    };
    let settings = super::shared::snapshot();
    let frame_rect: Rect = record.stashed_frame.into();
    let usable = frame_rect.right > frame_rect.left && frame_rect.bottom > frame_rect.top;
    let frame = if usable {
        let target_monitor = StashMonitor {
            monitor: StashRect::from(snapshot.monitor),
            work: StashRect::from(snapshot.work),
            dpi_x: snapshot.dpi_x,
            dpi_y: snapshot.dpi_y,
        };
        rebase_rect(frame_rect, &record.original_monitor, &target_monitor)
    } else {
        calculate_stashed_frame(snapshot.work, current, record.edge, settings.stash_peek)
    };
    if !native::set_pos(native_hwnd, frame) {
        if !native::set_placement(native_hwnd, &previous) {
            persist_remove(&record.id);
        }
        return;
    }
    let identity = match native::window_identity(native_hwnd) {
        Some(identity) => identity,
        None => return,
    };
    let monitor = StashMonitor {
        monitor: StashRect::from(snapshot.monitor),
        work: StashRect::from(snapshot.work),
        dpi_x: snapshot.dpi_x,
        dpi_y: snapshot.dpi_y,
    };
    {
        let Ok(mut guard) = live().lock() else {
            return;
        };
        guard.by_hwnd.insert(
            hwnd,
            LiveRecord {
                edge: record.edge,
                placement: placement_from_stash(&record.original_placement),
                monitor,
                identity: identity.clone(),
                persisted_id: record.id.clone(),
            },
        );
        guard.order.retain(|id| *id != hwnd);
        guard.order.push(hwnd);
    }
    persist_refresh_identity(
        &record.id,
        &identity.executable_path,
        identity.process_id,
        &identity.window_class,
        &identity.title,
    );
}

// --- persistence helpers (shared settings + atomic save) ---

fn persist_upsert(record: &StashRecord) {
    super::shared::update(|settings| {
        if let Some(slot) = settings
            .stash_records
            .iter_mut()
            .find(|r| r.id == record.id)
        {
            *slot = record.clone();
        } else {
            settings.stash_records.push(record.clone());
        }
    });
    super::shared::save_now();
}

fn persist_remove(id: &str) {
    super::shared::update(|settings| {
        settings.stash_records.retain(|r| r.id != id);
    });
    super::shared::save_now();
}

fn persist_refresh_identity(id: &str, exe: &str, pid: u32, class: &str, title: &str) {
    super::shared::update(|settings| {
        if let Some(slot) = settings.stash_records.iter_mut().find(|r| r.id == id) {
            slot.executable_path = exe.to_string();
            slot.process_id = pid;
            slot.window_class = class.to_string();
            slot.title = title.to_string();
        }
    });
    super::shared::save_now();
}

fn live_monitor(hwnd: u64) -> StashMonitor {
    live()
        .lock()
        .ok()
        .and_then(|guard| guard.by_hwnd.get(&hwnd).map(|r| r.monitor.clone()))
        .unwrap_or_default()
}

fn edge_name(edge: StashEdge) -> &'static str {
    match edge {
        StashEdge::Left => "left",
        StashEdge::Right => "right",
        StashEdge::Top => "top",
        StashEdge::Bottom => "bottom",
    }
}

fn in_hit_zone(cursor: Point, edge: StashEdge, work: Rect, zone: i32) -> bool {
    match edge {
        StashEdge::Left => {
            cursor.x >= work.left
                && cursor.x <= work.left + zone
                && cursor.y >= work.top
                && cursor.y <= work.bottom
        }
        StashEdge::Right => {
            cursor.x >= work.right - zone
                && cursor.x <= work.right
                && cursor.y >= work.top
                && cursor.y <= work.bottom
        }
        StashEdge::Top => {
            cursor.y >= work.top
                && cursor.y <= work.top + zone
                && cursor.x >= work.left
                && cursor.x <= work.right
        }
        StashEdge::Bottom => {
            cursor.y >= work.bottom - zone
                && cursor.y <= work.bottom
                && cursor.x >= work.left
                && cursor.x <= work.right
        }
    }
}

fn placement_to_stash(placement: &WINDOWPLACEMENT) -> StashPlacement {
    StashPlacement {
        length: placement.length as i32,
        flags: placement.flags.0,
        show_command: placement.showCmd,
        min_position: StashPoint {
            x: placement.ptMinPosition.x,
            y: placement.ptMinPosition.y,
        },
        max_position: StashPoint {
            x: placement.ptMaxPosition.x,
            y: placement.ptMaxPosition.y,
        },
        normal_position: StashRect::from(native::rect_from_native(placement.rcNormalPosition)),
    }
}

fn placement_from_stash(placement: &StashPlacement) -> WINDOWPLACEMENT {
    let normal: Rect = placement.normal_position.into();
    WINDOWPLACEMENT {
        length: core::mem::size_of::<WINDOWPLACEMENT>() as u32,
        flags: WINDOWPLACEMENT_FLAGS(placement.flags),
        showCmd: placement.show_command,
        ptMinPosition: windows::Win32::Foundation::POINT {
            x: placement.min_position.x,
            y: placement.min_position.y,
        },
        ptMaxPosition: windows::Win32::Foundation::POINT {
            x: placement.max_position.x,
            y: placement.max_position.y,
        },
        rcNormalPosition: native::rect_to_native(normal),
    }
}
