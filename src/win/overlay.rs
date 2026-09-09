//! Overlay window specs: titles, sizing, click-through behavior.
//!
//! The radial and preview overlays are iced windows (opened with
//! transparency + AlwaysOnTop + no decorations). After opening, the HWND
//! is found by title and given WS_EX_TRANSPARENT (preview only — the
//! radial overlay keeps clicks so press-to-commit works) plus
//! WS_EX_TOOLWINDOW so neither appears in the taskbar or Alt+Tab.

use super::native;
use crate::core::rect::Rect;

/// Unique titles used to find overlay HWNDs for style patching.
pub const RADIAL_TITLE: &str = "LoopW Radial";
pub const PREVIEW_TITLE: &str = "LoopW Preview";

/// Radial overlay square: cursor at center, ring + margin.
pub fn radial_bounds(cursor: crate::core::rect::Point, outer_radius: f64) -> Rect {
    let half = (outer_radius + 16.0).round() as i32;
    Rect::new(
        cursor.x - half,
        cursor.y - half,
        cursor.x + half,
        cursor.y + half,
    )
}

/// Physical pixels -> logical `(x, y, w, h)` for iced window sizing and
/// positioning. iced speaks logical coordinates; cursors, `GetWindowRect`,
/// and hover math are physical. Every overlay crosses this boundary once,
/// here, so the two systems can never drift apart on scaled displays.
pub fn to_logical(frame: Rect, scale: f64) -> (f32, f32, f32, f32) {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale as f32
    } else {
        1.0
    };
    (
        frame.left as f32 / scale,
        frame.top as f32 / scale,
        frame.width() as f32 / scale,
        frame.height() as f32 / scale,
    )
}

/// Apply click-through (+ tool-window) style to an overlay by title.
/// Retried by the caller for a short window after `window::open`.
pub fn make_click_through_by_title(title: &str) -> bool {
    match native::find_window_by_title(title) {
        Some(hwnd) => {
            native::make_overlay_click_through(hwnd);
            true
        }
        None => false,
    }
}

/// Find one of our own top-level windows whose rect matches `expected`
/// (±2 px) and add WS_EX_TOOLWINDOW (no taskbar button, no Alt+Tab).
pub fn patch_tool_window(expected: Rect) -> bool {
    match find_own_window(expected) {
        Some(hwnd) => {
            use windows::Win32::UI::WindowsAndMessaging::*;
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TOOLWINDOW.0 as isize);
            }
            true
        }
        None => false,
    }
}

/// Keep the radial menu above the target preview in the topmost band.
///
/// Both overlays open `AlwaysOnTop`, and the preview opens (and moves) after
/// the radial, so without this the preview settles on top and hides the
/// radial where they overlap. Inserts the preview directly below the radial
/// with `SWP_NOACTIVATE` so no focus is stolen; safe to call every frame
/// (same-order `SetWindowPos` is a no-op visually). Retried by the caller
/// because window rects lag a tick behind async open/move tasks.
pub fn order_preview_below_radial(preview_expected: Rect, radial_expected: Rect) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::*;
    let (Some(preview), Some(radial)) = (
        find_own_window(preview_expected),
        find_own_window(radial_expected),
    ) else {
        return false;
    };
    if preview == radial {
        return true;
    }
    unsafe {
        SetWindowPos(
            preview,
            Some(radial),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        )
        .is_ok()
    }
}

/// Find one of our own overlay windows at `expected` and make it
/// click-through (+ tool-window).
pub fn patch_click_through(expected: Rect) -> bool {
    match find_own_window(expected) {
        Some(hwnd) => {
            native::make_overlay_click_through(hwnd);
            true
        }
        None => false,
    }
}

/// Move and resize the live preview in its physical coordinate space.
///
/// Preview frames come from Win32 monitor/window geometry, while iced's
/// window effects use logical coordinates. Updating both through one native
/// `SetWindowPos` call prevents a resize and move from landing in different
/// compositor passes when the hover changes between differently sized zones.
pub fn set_preview_frame(frame: Rect) -> bool {
    native::find_window_by_title(PREVIEW_TITLE)
        .map(|hwnd| native::set_pos(hwnd, frame))
        .unwrap_or(false)
}

fn find_own_window(expected: Rect) -> Option<windows::Win32::Foundation::HWND> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    struct Pack {
        own_pid: u32,
        expected: Rect,
        found: Option<HWND>,
    }
    unsafe extern "system" fn proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let pack = &mut *(lparam.0 as *mut Pack);
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != pack.own_pid {
            return BOOL::from(true);
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return BOOL::from(true);
        }
        let frame = native::rect_from_native(rect);
        let expected = pack.expected;
        let close = (frame.left - expected.left).abs() <= 2
            && (frame.top - expected.top).abs() <= 2
            && (frame.width() - expected.width()).abs() <= 2
            && (frame.height() - expected.height()).abs() <= 2;
        if close {
            pack.found = Some(hwnd);
            return BOOL::from(false);
        }
        BOOL::from(true)
    }
    let mut pack = Pack {
        own_pid: native::own_process_id(),
        expected,
        found: None,
    };
    unsafe {
        let _ = EnumWindows(Some(proc), LPARAM(&mut pack as *mut Pack as isize));
    }
    pack.found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_round_trip_preserves_physical_within_one_px() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let frame = Rect::new(100, 100, 100 + 961, 100 + 543);
            let (x, y, w, h) = to_logical(frame, scale);
            let back_w = (w * scale as f32).round() as i32;
            let back_h = (h * scale as f32).round() as i32;
            assert!(
                (back_w - frame.width()).abs() <= 1,
                "scale {scale}: width drifted {back_w} vs {}",
                frame.width()
            );
            assert!(
                (back_h - frame.height()).abs() <= 1,
                "scale {scale}: height drifted {back_h} vs {}",
                frame.height()
            );
            assert_eq!((x * scale as f32).round() as i32, frame.left);
            assert_eq!((y * scale as f32).round() as i32, frame.top);
        }
    }

    #[test]
    fn degenerate_scale_falls_back_to_identity() {
        let frame = Rect::new(0, 0, 100, 100);
        assert_eq!(to_logical(frame, 0.0), (0.0, 0.0, 100.0, 100.0));
        assert_eq!(to_logical(frame, f64::NAN), (0.0, 0.0, 100.0, 100.0));
    }
}
