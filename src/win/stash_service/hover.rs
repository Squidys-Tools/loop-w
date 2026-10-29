//! Hover reveal: throttled cursor polling, work-area edge hit zones,
//! and reveal-delay arming.

use crate::core::rect::{Point, Rect};
use crate::core::stash::StashEdge;
use crate::win::{monitor_service, native, shared};

use super::live;
use super::records::prune_stale;
use super::reveal::reveal;

const POLL_INTERVAL_MS: u64 = 80;

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

/// Whether any stashed window needs cursor hover polling.
pub fn is_active() -> bool {
    live()
        .lock()
        .map(|guard| !guard.by_hwnd.is_empty())
        .unwrap_or(false)
}

/// Reveal whichever stashed window's hit zone holds the cursor.
pub fn reveal_at_cursor(cursor: Point) -> Option<String> {
    let settings = shared::snapshot();
    let zone = settings.stash_hit_zone.clamp(1, 96);
    let delay = settings.stash_reveal_delay_ms.clamp(0, 2000) as u64;
    let hit = {
        let guard = live().lock().ok()?;
        guard.order.iter().find_map(|hwnd| {
            let record = guard.by_hwnd.get(hwnd)?;
            // Resolve the CURRENT monitor from the stashed frame so
            // display/padding/work-area changes move the hit zone.
            let work = monitor_service::for_rect(record.frame)
                .map(|snapshot| snapshot.work)
                .unwrap_or_else(|| record.monitor.work.into());
            in_hit_zone(cursor, record.edge, work, zone).then_some(*hwnd)
        })
    };
    let Some(hwnd) = hit else {
        if let Ok(mut guard) = live().lock() {
            guard.pending = None;
        }
        return None;
    };
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
                reveal(hwnd).ok()
            } else {
                None
            }
        }
        _ => {
            guard.pending = Some((hwnd, now));
            None
        }
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
