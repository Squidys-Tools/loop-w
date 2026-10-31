//! Reveal paths: FIFO reveal-next, hover-targeted reveal, and the
//! restore-everything pass that runs on clean quit.

use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::rect::Rect;
use crate::core::stash::{StashMonitor, StashRect};
use crate::core::stash_rebase::{find_restore_monitor, rebase_rect};
use crate::win::{monitor_service, native, policy, shared};

use super::convert::placement_to_stash;
use super::records::{is_stash_alive, prune_stale, remove_runtime, reset_pending};
use super::{live, LiveRecord};

/// Reveal the first stashed window in FIFO order (hotkey/command path).
pub fn reveal_next() -> Result<String, String> {
    prune_stale();
    reset_pending();
    let target = live()
        .lock()
        .ok()
        .and_then(|guard| guard.order.iter().copied().next());
    match target {
        Some(hwnd) => reveal(hwnd),
        None => Err("No stashed windows to reveal.".to_string()),
    }
}

pub(super) fn reveal(hwnd: u64) -> Result<String, String> {
    let record = live()
        .lock()
        .ok()
        .and_then(|guard| guard.by_hwnd.get(&hwnd).map(clone_live))
        .ok_or_else(|| "The target window is not stashed.".to_string())?;
    if !policy::is_eligible_for_enumeration(hwnd as isize, 0) {
        return Err("The stashed window is excluded or no longer an eligible target.".to_string());
    }
    let monitors = monitor_service::all();
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
        // Drop only when the window is truly gone (full aliveness check:
        // PID reuse or exe change must not strand retries on a live HWND).
        if !is_stash_alive(hwnd) {
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

/// Restore every alive window (clean quit): nothing stays off-screen.
/// Persisted records survive a clean quit (only a disabled persistence
/// setting clears them); exactly one save happens iff anything changed.
pub fn restore_all() {
    let hwnds: Vec<u64> = live()
        .lock()
        .map(|guard| guard.order.clone())
        .unwrap_or_default();
    let mut changed = false;
    for hwnd in hwnds {
        // reveal_quiet with persist=false keeps file records for next launch.
        changed |= reveal_quiet_keep(hwnd);
    }
    reset_pending();
    if !shared::snapshot().stash_persistence_enabled {
        let had_records = !shared::snapshot().stash_records.is_empty();
        if had_records {
            shared::update(|settings| {
                settings.stash_records.clear();
            });
            changed = true;
        }
    }
    // Drop any leftover runtime entries (dead windows) without touching disk.
    if let Ok(mut guard) = live().lock() {
        if !guard.by_hwnd.is_empty() || !guard.order.is_empty() {
            guard.by_hwnd.clear();
            guard.order.clear();
            changed = true;
        }
    }
    if changed {
        shared::save_now();
    }
}

fn reveal_quiet_keep(hwnd: u64) -> bool {
    let record = live()
        .lock()
        .ok()
        .and_then(|guard| guard.by_hwnd.get(&hwnd).map(clone_live));
    let Some(record) = record else {
        remove_runtime(hwnd, false);
        return false;
    };
    let monitors = monitor_service::all();
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
    remove_runtime(hwnd, false);
    ok
}
