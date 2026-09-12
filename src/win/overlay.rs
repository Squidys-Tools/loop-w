//! Overlay window specs: titles, sizing, click-through behavior.
//!
//! The radial and preview overlays are iced windows (opened with
//! transparency + AlwaysOnTop + no decorations). After opening, the HWND
//! is found by title and given WS_EX_TRANSPARENT (preview only — the
//! radial overlay keeps clicks so press-to-commit works) plus
//! WS_EX_TOOLWINDOW so neither appears in the taskbar or Alt+Tab.

use super::native;
use crate::core::rect::Rect;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy)]
enum CacheSlot {
    Radial,
    Preview,
}

#[derive(Default)]
struct OverlayCache {
    radial: Option<isize>,
    preview: Option<isize>,
    ordered: Option<(isize, isize)>,
}

static OVERLAY_CACHE: OnceLock<Mutex<OverlayCache>> = OnceLock::new();

fn overlay_cache() -> &'static Mutex<OverlayCache> {
    OVERLAY_CACHE.get_or_init(|| Mutex::new(OverlayCache::default()))
}

fn cached_window(slot: CacheSlot, expected: Rect) -> Option<windows::Win32::Foundation::HWND> {
    let Ok(mut cache) = overlay_cache().lock() else {
        return find_own_window(slot, expected);
    };
    let cached = match slot {
        CacheSlot::Radial => &mut cache.radial,
        CacheSlot::Preview => &mut cache.preview,
    };
    if let Some(raw) = *cached {
        let hwnd = native::from_raw(raw);
        if window_matches(hwnd, expected) {
            return Some(hwnd);
        }
        *cached = None;
    }
    let found = find_own_window(slot, expected);
    *cached = found.map(native::raw);
    found
}

fn window_matches(hwnd: windows::Win32::Foundation::HWND, expected: Rect) -> bool {
    native::is_window(hwnd)
        && native::process_id(hwnd) == native::own_process_id()
        && native::window_rect(hwnd).is_some_and(|frame| {
            (frame.left - expected.left).abs() <= 2
                && (frame.top - expected.top).abs() <= 2
                && (frame.width() - expected.width()).abs() <= 2
                && (frame.height() - expected.height()).abs() <= 2
        })
}

/// Unique titles used to find overlay HWNDs for style patching.
pub const RADIAL_TITLE: &str = "LoopW Radial";
pub const PREVIEW_TITLE: &str = "LoopW Preview";
pub const TRAY_MENU_TITLE: &str = "LoopW Tray Menu";

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
    let scale = valid_scale(scale);
    (
        frame.left as f32 / scale,
        frame.top as f32 / scale,
        frame.width() as f32 / scale,
        frame.height() as f32 / scale,
    )
}

/// Convert a physical target frame into logical coordinates relative to a
/// fixed physical overlay frame.
pub fn to_local_logical(target: Rect, overlay: Rect, scale: f64) -> (f32, f32, f32, f32) {
    let scale = valid_scale(scale);
    (
        (target.left - overlay.left) as f32 / scale,
        (target.top - overlay.top) as f32 / scale,
        target.width() as f32 / scale,
        target.height() as f32 / scale,
    )
}

fn valid_scale(scale: f64) -> f32 {
    if scale.is_finite() && scale > 0.0 {
        scale as f32
    } else {
        1.0
    }
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
    match cached_window(CacheSlot::Radial, expected) {
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

/// Add WS_EX_TOOLWINDOW to a short-lived app-owned popup that is not an
/// overlay and therefore has no stable physical frame cache.
pub fn patch_tool_window_by_title(title: &str) -> bool {
    let Some(hwnd) = native::find_window_by_title(title) else {
        return false;
    };
    if native::process_id(hwnd) != native::own_process_id() {
        return false;
    }
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_TOOLWINDOW.0 as isize);
    }
    true
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
        cached_window(CacheSlot::Preview, preview_expected),
        cached_window(CacheSlot::Radial, radial_expected),
    ) else {
        return false;
    };
    if preview == radial {
        return true;
    }
    let order = (native::raw(preview), native::raw(radial));
    if overlay_cache()
        .lock()
        .map(|cache| cache.ordered == Some(order))
        .unwrap_or(false)
    {
        return true;
    }
    let ordered = unsafe {
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
    };
    if ordered {
        if let Ok(mut cache) = overlay_cache().lock() {
            cache.ordered = Some(order);
        }
    }
    ordered
}

/// Find one of our own overlay windows at `expected` and make it
/// click-through (+ tool-window).
pub fn patch_click_through(expected: Rect) -> bool {
    // The preview can be created after its target frame has already moved.
    // During that short async window its HWND may not yet match the cached
    // physical rect, so fall back to the unique app-owned title instead of
    // exhausting the retry budget and leaving a taskbar/Alt+Tab window.
    let hwnd = cached_window(CacheSlot::Preview, expected).or_else(|| {
        native::find_window_by_title(PREVIEW_TITLE)
            .filter(|hwnd| native::process_id(*hwnd) == native::own_process_id())
    });
    match hwnd {
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
    cached_window(CacheSlot::Preview, frame)
        .map(|hwnd| native::set_pos(hwnd, frame))
        .unwrap_or(false)
}

fn find_own_window(slot: CacheSlot, expected: Rect) -> Option<windows::Win32::Foundation::HWND> {
    let title = match slot {
        CacheSlot::Radial => RADIAL_TITLE,
        CacheSlot::Preview => PREVIEW_TITLE,
    };
    let hwnd = native::find_window_by_title(title)?;
    window_matches(hwnd, expected).then_some(hwnd)
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

    #[test]
    fn target_zone_stays_inside_fixed_overlay_after_transition() {
        let overlay = Rect::new(0, 0, 1920, 1080);
        let quarter = Rect::new(0, 0, 960, 540);
        let half = Rect::new(0, 0, 1920, 540);
        assert_eq!(
            to_local_logical(quarter, overlay, 1.0),
            (0.0, 0.0, 960.0, 540.0)
        );
        assert_eq!(
            to_local_logical(half, overlay, 1.0),
            (0.0, 0.0, 1920.0, 540.0)
        );
        assert!(half.right <= overlay.right && half.bottom <= overlay.bottom);
    }

    #[test]
    fn local_target_translation_handles_dpi() {
        let overlay = Rect::new(1920, 0, 3840, 1080);
        let target = Rect::new(2880, 540, 3840, 1080);
        assert_eq!(
            to_local_logical(target, overlay, 2.0),
            (480.0, 270.0, 480.0, 270.0)
        );
    }
}
