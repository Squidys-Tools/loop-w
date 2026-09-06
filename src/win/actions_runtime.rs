//! Live action execution: the `TryApply` dispatch.
//!
//! Ports `WindowActionService.TryApply` ordering exactly: RevealStashed
//! first, window/policy gates, special handlers, then the geometry path
//! (target frame → monitor bounds → undo → fit → place → re-anchor).

use windows::Win32::UI::WindowsAndMessaging::*;

use super::native;
use super::placement;
use super::policy;
use super::target_frame;
use crate::core::actions::WindowAction;
use crate::core::frame_math::{MinMaxLimits, fit_frame};
use crate::core::nav::{NavDir, find_directional, find_next_in_stack};
use crate::core::rect::Rect;

/// Apply an action. `Ok(message)` describes what happened.
pub fn apply(hwnd: u64, action: WindowAction) -> Result<String, String> {
    if action == WindowAction::RevealStashed {
        return super::stash_service::reveal_next();
    }
    if hwnd == 0 || !native::is_window(native::from_raw(hwnd as isize)) {
        return Err("The target window is no longer available.".to_string());
    }
    if let Err(diagnostic) = policy::try_authorize_action(hwnd, action) {
        return Err(diagnostic.to_string());
    }
    match action {
        WindowAction::Minimize => show_and_report(hwnd, SW_MINIMIZE, "Minimize"),
        WindowAction::Hide => show_and_report(hwnd, SW_HIDE, "Hide"),
        WindowAction::MinimizeOthers => minimize_others(hwnd),
        WindowAction::FocusUp | WindowAction::FocusDown | WindowAction::FocusLeft | WindowAction::FocusRight => {
            focus_directional(hwnd, action)
        }
        WindowAction::FocusNextInStack => focus_next(hwnd),
        WindowAction::Stash => super::stash_service::stash(hwnd),
        WindowAction::RestoreInitialFrame => placement::restore_initial_frame(hwnd),
        WindowAction::Undo => placement::undo(hwnd),
        _ => apply_geometry_here(hwnd, action),
    }
}

/// Geometry path shared by zones, maximize variants, screens, nudges.
fn apply_geometry_here(hwnd: u64, action: WindowAction) -> Result<String, String> {
    apply_geometry(hwnd, action, None)
}

/// Geometry path with a precomputed ideal frame (snap commit).
pub fn apply_geometry(hwnd: u64, action: WindowAction, ideal: Option<Rect>) -> Result<String, String> {
    let ideal = match ideal {
        Some(frame) => frame,
        None => target_frame::target_frame(hwnd, action)?,
    };
    let snapshot = super::monitor_service::for_rect(ideal)
        .ok_or_else(|| "Could not determine the target monitor.".to_string())?;
    let bounds = if action == WindowAction::Fullscreen {
        snapshot.monitor
    } else {
        snapshot.work
    };
    placement::push_undo(hwnd);
    let frame = fit_frame(bounds, action, ideal, min_max_limits(hwnd));
    if !placement::place_window(hwnd, frame) {
        return Err(
            "Windows rejected the move. The target may be elevated, protected, or non-resizable."
                .to_string(),
        );
    }
    let actual = native::window_rect(native::from_raw(hwnd as isize))
        .ok_or_else(|| "Could not read the window's final position.".to_string())?;
    // Re-anchor: apps that clamp to min-size without honoring the anchor.
    let mut actual = actual;
    if !placement::rects_equal(actual, frame) {
        let reanchored = fit_frame(
            bounds,
            action,
            actual,
            MinMaxLimits { min_w: 0, min_h: 0, max_w: 0, max_h: 0 },
        );
        if !placement::rects_equal(reanchored, frame) {
            let _ = placement::place_window(hwnd, reanchored);
            if let Some(updated) = native::window_rect(native::from_raw(hwnd as isize)) {
                actual = updated;
            }
        }
    }
    let label = action.display_name();
    if sizes_equal(actual, ideal) {
        Ok(format!("Applied {label} to target window"))
    } else {
        Ok(format!(
            "Snapped to {label}, but the window's minimum/maximum size forced {}×{} instead of {}×{}.",
            actual.width(),
            actual.height(),
            ideal.width(),
            ideal.height()
        ))
    }
}

/// Snap commit: re-validated from scratch against the ideal frame.
pub fn apply_snap(hwnd: u64, action: WindowAction, frame: Rect) -> Result<String, String> {
    if hwnd == 0
        || !native::is_window(native::from_raw(hwnd as isize))
        || !super::query::is_eligible_for_snap(hwnd)
    {
        return Err("The dragged window is no longer available.".to_string());
    }
    if let Err(diagnostic) = policy::try_authorize_action(hwnd, action) {
        return Err(diagnostic.to_string());
    }
    let snapshot = super::monitor_service::for_rect(frame)
        .ok_or_else(|| "Could not determine the snap monitor.".to_string())?;
    let fitted = fit_frame(snapshot.work, action, frame, min_max_limits(hwnd));
    placement::push_undo(hwnd);
    if !placement::place_window(hwnd, fitted) {
        return Err(
            "Windows rejected the snap. The target may be elevated, protected, or non-resizable."
                .to_string(),
        );
    }
    let actual = native::window_rect(native::from_raw(hwnd as isize))
        .ok_or_else(|| "Could not read the snapped window's final position.".to_string())?;
    let label = action.display_name();
    if sizes_equal(actual, frame) {
        Ok(format!("Applied drag snap: {label}"))
    } else {
        Ok(format!(
            "Snapped to {label}, but the window's minimum/maximum size forced {}×{} instead of {}×{}.",
            actual.width(),
            actual.height(),
            frame.width(),
            frame.height()
        ))
    }
}

/// Pre-drag-frame restore for cancelled snaps.
pub fn restore_frame(hwnd: u64, frame: Rect) -> Result<String, String> {
    if hwnd == 0 || !native::is_window(native::from_raw(hwnd as isize)) {
        return Err("The dragged window is no longer available.".to_string());
    }
    // MoveRight never requires resize: the proxy keeps policy truthful.
    if let Err(diagnostic) = policy::try_authorize_action(hwnd, WindowAction::MoveRight) {
        return Err(diagnostic.to_string());
    }
    if !placement::place_window(hwnd, frame) {
        return Err("Could not restore the pre-drag frame.".to_string());
    }
    match native::window_rect(native::from_raw(hwnd as isize)) {
        Some(actual) if placement::rects_equal(actual, frame) => {
            Ok("Restored the pre-drag frame".to_string())
        }
        Some(_) => Ok("The application did not accept the pre-drag frame.".to_string()),
        None => Err("Could not restore the pre-drag frame.".to_string()),
    }
}

fn show_and_report(hwnd: u64, cmd: SHOW_WINDOW_CMD, label: &str) -> Result<String, String> {
    if !native::show_window(native::from_raw(hwnd as isize), cmd) {
        return Err("The target window rejected the command.".to_string());
    }
    Ok(format!("Applied {label} to target window"))
}

fn minimize_others(hwnd: u64) -> Result<String, String> {
    let mut minimized = 0u32;
    for candidate in super::query::enumerate(hwnd) {
        native::show_window(native::from_raw(candidate.hwnd as isize), SW_MINIMIZE);
        if native::is_iconic(native::from_raw(candidate.hwnd as isize)) {
            minimized += 1;
        }
    }
    if minimized == 0 {
        return Err("No other eligible windows were found.".to_string());
    }
    Ok(format!(
        "Minimized {minimized} other window{}",
        if minimized == 1 { "" } else { "s" }
    ))
}

fn focus_directional(hwnd: u64, action: WindowAction) -> Result<String, String> {
    let dir = match action {
        WindowAction::FocusLeft => NavDir::Left,
        WindowAction::FocusRight => NavDir::Right,
        WindowAction::FocusUp => NavDir::Up,
        _ => NavDir::Down,
    };
    let source = native::window_rect(native::from_raw(hwnd as isize))
        .ok_or_else(|| "The target window is no longer available.".to_string())?;
    let candidates: Vec<(u64, Rect)> = super::query::enumerate(hwnd)
        .into_iter()
        .map(|candidate| (candidate.hwnd, candidate.frame))
        .collect();
    let Some(target) = find_directional(source, &candidates, dir) else {
        let name = action.display_name();
        let short = name.strip_prefix("Focus ").unwrap_or(name).to_lowercase();
        return Err(format!("No eligible window found {short}."));
    };
    focus_window(target, action.display_name())
}

fn focus_next(hwnd: u64) -> Result<String, String> {
    let order: Vec<u64> = super::query::enumerate(hwnd)
        .into_iter()
        .map(|candidate| candidate.hwnd)
        .collect();
    let Some(target) = find_next_in_stack(&order, hwnd) else {
        return Err("No other eligible window was found in the window stack.".to_string());
    };
    focus_window(target, WindowAction::FocusNextInStack.display_name())
}

fn focus_window(hwnd: u64, label: &str) -> Result<String, String> {
    let native_hwnd = native::from_raw(hwnd as isize);
    if native::is_iconic(native_hwnd) {
        native::show_window(native_hwnd, SW_RESTORE);
    }
    if !native::set_foreground(native_hwnd) {
        return Err("Windows rejected the focus request.".to_string());
    }
    Ok(format!("Applied {label} to target window"))
}

fn min_max_limits(hwnd: u64) -> MinMaxLimits {
    let info = native::min_max_info(native::from_raw(hwnd as isize));
    MinMaxLimits {
        min_w: info.ptMinTrackSize.x,
        min_h: info.ptMinTrackSize.y,
        max_w: info.ptMaxTrackSize.x,
        max_h: info.ptMaxTrackSize.y,
    }
}

fn sizes_equal(first: Rect, second: Rect) -> bool {
    (first.width() - second.width()).abs() <= 2
        && (first.height() - second.height()).abs() <= 2
}
