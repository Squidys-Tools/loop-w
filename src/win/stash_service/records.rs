//! Live-record bookkeeping: stale-entry pruning, aliveness checks with a
//! PID-reuse guard, runtime removal, and pending-reveal reset.

use crate::core::identity::WindowIdentity;
use crate::win::{diagnostics, native};

use super::live;
use super::persist::persist_remove;

/// Clear any armed hover-reveal (settings change, explicit reveal, exit).
pub(super) fn reset_pending() {
    if let Ok(mut guard) = live().lock() {
        guard.pending = None;
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
                .filter(|hwnd| {
                    guard
                        .by_hwnd
                        .get(hwnd)
                        .map(|record| !is_stash_alive_with_identity(*hwnd, &record.identity))
                        .unwrap_or(true)
                })
                .collect()
        })
        .unwrap_or_default();
    if dead.is_empty() {
        return;
    }
    diagnostics::report_stash_stale(dead.len());
    for hwnd in dead {
        remove_runtime(hwnd, true);
    }
}

pub(super) fn is_stash_alive(hwnd: u64) -> bool {
    let identity = live()
        .lock()
        .ok()
        .and_then(|guard| guard.by_hwnd.get(&hwnd).map(|r| r.identity.clone()));
    identity
        .as_ref()
        .is_some_and(|identity| is_stash_alive_with_identity(hwnd, identity))
}

fn is_stash_alive_with_identity(hwnd: u64, identity: &WindowIdentity) -> bool {
    let native_hwnd = native::from_raw(hwnd as isize);
    if hwnd == 0 || !native::is_window(native_hwnd) {
        return false;
    }
    let pid = native::process_id(native_hwnd);
    if pid == 0 {
        return false;
    }
    if identity.process_id != pid {
        // PID reused by an unrelated process: verify class still matches.
        let class = native::window_class(native_hwnd);
        if identity.window_class.is_empty() || !identity.window_class.eq_ignore_ascii_case(&class) {
            return false;
        }
    }
    true
}

pub(super) fn remove_runtime(hwnd: u64, persist: bool) {
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
