//! Persistence: stash records live in shared settings with atomic saves,
//! startup re-stash of unambiguous matches, and the settings-change hook.

use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::rect::Rect;
use crate::core::stash::{calculate_stashed_frame, StashMonitor, StashRecord, StashRect};
use crate::core::stash_rebase::rebase_rect;
use crate::win::{monitor_service, native, query, shared};

use super::convert::placement_from_stash;
use super::records::reset_pending;
use super::{live, LiveRecord};

/// Settings changed: drop armed reveals; disabling persistence clears
/// persisted records (matching Configure/UpdateSettings).
pub fn handle_settings_changed() {
    reset_pending();
    let enabled = shared::snapshot().stash_persistence_enabled;
    if !enabled {
        let had_records = shared::snapshot().stash_records.is_empty();
        if !had_records {
            shared::update(|settings| {
                settings.stash_records.clear();
            });
            shared::save_now();
        }
    }
}

/// Re-stash persisted records whose live window matches unambiguously.
pub fn restore_persisted() {
    use crate::core::identity::{find_unambiguous_match, WindowIdentity};
    if !shared::snapshot().stash_persistence_enabled {
        return;
    }
    let records = shared::snapshot().stash_records;
    if records.is_empty() {
        return;
    }
    let mut claimed: Vec<u64> = Vec::new();
    for record in &records {
        // Enumerate once (restore scan keeps minimized/zero-area windows).
        let candidates: Vec<(u64, WindowIdentity)> = query::enumerate_for_restore()
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
        .and_then(|rect| monitor_service::for_rect(rect).map(|s| (rect, s)))
    {
        Some(pair) => pair,
        None => {
            // Uncontrollable window: delete the record so it cannot act
            // on every launch when even the rollback fails.
            if native::set_placement(native_hwnd, &previous) {
                return;
            }
            persist_remove(&record.id);
            return;
        }
    };
    let settings = shared::snapshot();
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
                frame,
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

pub(super) fn persist_upsert(record: &StashRecord) {
    shared::update(|settings| {
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
    shared::save_now();
}

pub(super) fn persist_remove(id: &str) {
    shared::update(|settings| {
        settings.stash_records.retain(|r| r.id != id);
    });
    shared::save_now();
}

fn persist_refresh_identity(id: &str, exe: &str, pid: u32, class: &str, title: &str) {
    shared::update(|settings| {
        if let Some(slot) = settings.stash_records.iter_mut().find(|r| r.id == id) {
            slot.executable_path = exe.to_string();
            slot.process_id = pid;
            slot.window_class = class.to_string();
            slot.title = title.to_string();
        }
    });
    shared::save_now();
}
