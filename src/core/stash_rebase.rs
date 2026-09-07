//! Stash frame rebasing across DPI / work-area / monitor changes.
//!
//! Ports `WindowStashService.RebaseRect` + restore-monitor lookup. Only the
//! saved `NormalPosition` is rebased; flags, `ShowCmd`, and min/max positions
//! pass through untouched.

use super::monitor::MonitorSnapshot;
use super::rect::Rect;
use super::stash::StashMonitor;

/// Rebase a saved frame from the recorded monitor onto the current one.
pub fn rebase_rect(frame: Rect, original: &StashMonitor, target: &StashMonitor) -> Rect {
    let orig_work: Rect = original.work.into();
    let target_work: Rect = target.work.into();
    if !is_usable(orig_work) || !is_usable(target_work) {
        return frame;
    }
    let orig_monitor: Rect = original.monitor.into();
    let target_monitor: Rect = target.monitor.into();
    if orig_monitor == target_monitor && orig_work == target_work {
        // Same monitor, same work area: WINDOWPLACEMENT coords are physical
        // and Windows already adjusted them on a DPI-only change.
        // (Degenerate recorded monitors take the DPI-scale path below.)
        return frame;
    }
    if orig_monitor == target_monitor && is_usable(orig_monitor) {
        // Same monitor, work area changed: translate by the origin delta.
        let dx = target_work.left - orig_work.left;
        let dy = target_work.top - orig_work.top;
        return Rect::new(
            frame.left + dx,
            frame.top + dy,
            frame.right + dx,
            frame.bottom + dy,
        );
    }
    // Different monitor: scale by the DPI ratio around the work origins.
    let sx = normalize_dpi(target.dpi_x) / normalize_dpi(original.dpi_x);
    let sy = normalize_dpi(target.dpi_y) / normalize_dpi(original.dpi_y);
    Rect::new(
        scale(frame.left - orig_work.left, sx) + target_work.left,
        scale(frame.top - orig_work.top, sy) + target_work.top,
        scale(frame.right - orig_work.left, sx) + target_work.left,
        scale(frame.bottom - orig_work.top, sy) + target_work.top,
    )
}

/// Pick the monitor to restore onto: exact full-or-work match wins,
/// otherwise the nearest monitor by Manhattan center distance.
pub fn find_restore_monitor<'a>(
    original: &StashMonitor,
    current: &'a [MonitorSnapshot],
) -> Option<&'a MonitorSnapshot> {
    let orig_monitor: Rect = original.monitor.into();
    let orig_work: Rect = original.work.into();
    for snapshot in current {
        if snapshot.monitor == orig_monitor || snapshot.work == orig_work {
            return Some(snapshot);
        }
    }
    if !is_usable(orig_monitor) {
        return None;
    }
    let (ox, oy) = center(orig_monitor);
    // Nearest by Manhattan center distance over all monitors (even
    // degenerate ones — C# orders the full list; empty list fails).
    current.iter().min_by_key(|s| {
        let (cx, cy) = center(s.monitor);
        ((cx - ox).abs() as i64) + ((cy - oy).abs() as i64)
    })
}

fn is_usable(rect: Rect) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

fn center(rect: Rect) -> (i32, i32) {
    (rect.left + rect.width() / 2, rect.top + rect.height() / 2)
}

fn normalize_dpi(dpi: f64) -> f64 {
    if dpi.is_finite() && dpi > 0.0 {
        dpi
    } else {
        96.0
    }
}

fn scale(value: i32, factor: f64) -> i32 {
    let scaled = value as f64 * factor;
    if scaled.is_finite() {
        scaled.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::stash::StashRect;

    fn monitor(monitor: Rect, work: Rect, dpi: f64) -> StashMonitor {
        StashMonitor {
            monitor: StashRect::from(monitor),
            work: StashRect::from(work),
            dpi_x: dpi,
            dpi_y: dpi,
        }
    }

    fn snapshot(monitor: Rect, work: Rect, dpi: f64) -> MonitorSnapshot {
        MonitorSnapshot {
            monitor,
            work,
            dpi_x: dpi,
            dpi_y: dpi,
        }
    }

    #[test]
    fn same_monitor_dpi_change_preserves_frame() {
        let original = monitor(
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1040),
            96.0,
        );
        let changed = monitor(
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1040),
            144.0,
        );
        let frame = Rect::new(100, 120, 700, 620);
        assert_eq!(rebase_rect(frame, &original, &changed), frame);
    }

    #[test]
    fn same_monitor_work_area_change_translates() {
        let original = monitor(
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1040),
            96.0,
        );
        let changed = monitor(
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 40, 1920, 1040),
            96.0,
        );
        assert_eq!(
            rebase_rect(Rect::new(100, 120, 700, 620), &original, &changed),
            Rect::new(100, 160, 700, 660)
        );
    }

    #[test]
    fn different_monitor_scales_by_dpi() {
        let original = monitor(
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1040),
            96.0,
        );
        let changed = monitor(
            Rect::new(1920, 0, 3840, 1080),
            Rect::new(1920, 0, 3840, 1040),
            144.0,
        );
        assert_eq!(
            rebase_rect(Rect::new(100, 100, 700, 600), &original, &changed),
            Rect::new(2070, 150, 2970, 900)
        );
    }

    #[test]
    fn restore_prefers_exact_match_then_nearest() {
        let original = monitor(
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1040),
            96.0,
        );
        let monitors = vec![
            snapshot(
                Rect::new(1920, 0, 3840, 1080),
                Rect::new(1920, 0, 3840, 1040),
                96.0,
            ),
            snapshot(
                Rect::new(0, 0, 1920, 1080),
                Rect::new(0, 0, 1920, 1040),
                96.0,
            ),
        ];
        let picked = find_restore_monitor(&original, &monitors).unwrap();
        assert_eq!(picked.monitor.left, 0);
    }
}
