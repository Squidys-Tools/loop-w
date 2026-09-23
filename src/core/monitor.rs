//! Monitor snapshots, screen padding, and cross-monitor translation.
//!
//! Ports `MonitorService.ApplyPadding` and `TranslateFrame` without Win32
//! calls so the math stays testable. Live enumeration lives in `crate::win`.

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::rect::Rect;

/// How a window keeps its size when moved across monitors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum MonitorMoveSizePolicy {
    #[default]
    PreservePixels = 0,
    PreserveLogicalSize = 1,
}

/// Padded work area plus DPI for one monitor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub monitor: Rect,
    pub work: Rect,
    pub dpi_x: f64,
    pub dpi_y: f64,
}

/// Combine global + per-edge padding and clamp it inside `work`.
pub fn apply_padding(
    work: Rect,
    global: i32,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
) -> Rect {
    let left = (global + left).max(0);
    let top = (global + top).max(0);
    let right = (global + right).max(0);
    let bottom = (global + bottom).max(0);

    let left = left.min(0.max(work.width() - 1));
    let right = right.min(0.max(work.width() - left - 1));
    let top = top.min(0.max(work.height() - 1));
    let bottom = bottom.min(0.max(work.height() - top - 1));

    let horizontal = 1.max(work.width() - left - right);
    let vertical = 1.max(work.height() - top - bottom);
    Rect::new(
        work.left + left,
        work.top + top,
        work.left + left + horizontal,
        work.top + top + vertical,
    )
}

/// Translate a frame from one monitor's work area to another's.
pub fn translate_frame(
    current: Rect,
    source: MonitorSnapshot,
    target: MonitorSnapshot,
    policy: MonitorMoveSizePolicy,
) -> Rect {
    let (scale_x, scale_y) = match policy {
        MonitorMoveSizePolicy::PreservePixels => (1.0, 1.0),
        MonitorMoveSizePolicy::PreserveLogicalSize => (
            normalize_dpi(target.dpi_x) / normalize_dpi(source.dpi_x),
            normalize_dpi(target.dpi_y) / normalize_dpi(source.dpi_y),
        ),
    };
    let left = target.work.left + scale_dim(current.left - source.work.left, scale_x);
    let top = target.work.top + scale_dim(current.top - source.work.top, scale_y);
    let width = 1.max(scale_dim(current.width(), scale_x));
    let height = 1.max(scale_dim(current.height(), scale_y));
    Rect::new(left, top, left + width, top + height)
}

fn scale_dim(value: i32, scale: f64) -> i32 {
    let scaled = value as f64 * scale;
    if scaled.is_finite() {
        scaled.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32
    } else {
        value
    }
}

fn normalize_dpi(dpi: f64) -> f64 {
    if dpi.is_finite() && dpi > 0.0 {
        dpi
    } else {
        96.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(monitor: Rect, work: Rect, dpi: f64) -> MonitorSnapshot {
        MonitorSnapshot {
            monitor,
            work,
            dpi_x: dpi,
            dpi_y: dpi,
        }
    }

    #[test]
    fn padding_combines_global_and_edges() {
        let work = Rect::new(0, 0, 1920, 1040);
        let padded = apply_padding(work, 8, 4, 0, 0, 0);
        assert_eq!(padded.left, 12);
        assert_eq!(padded.top, 8);
        assert!(padded.width() < work.width());
    }

    #[test]
    fn logical_moves_scale_with_dpi() {
        let source = snapshot(
            Rect::new(0, 0, 1920, 1080),
            Rect::new(0, 0, 1920, 1040),
            96.0,
        );
        let target = snapshot(
            Rect::new(1920, 0, 3840, 1080),
            Rect::new(1920, 0, 3840, 1040),
            144.0,
        );
        let current = Rect::new(100, 100, 500, 400);
        let moved = translate_frame(
            current,
            source,
            target,
            MonitorMoveSizePolicy::PreserveLogicalSize,
        );
        assert_eq!(moved.width(), 600);
        assert_eq!(moved.height(), 450);
        let pixels = translate_frame(
            current,
            source,
            target,
            MonitorMoveSizePolicy::PreservePixels,
        );
        assert_eq!((pixels.width(), pixels.height()), (400, 300));
    }
}
