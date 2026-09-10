//! Window authorization policy: which windows LoopW may touch.
//!
//! Ports `WindowPolicy.Evaluate` / `TryAuthorizeAction` /
//! `IsEligibleForEnumeration` with live Win32 checks. Exclusion sets come
//! from the shared settings snapshot.

use windows::Win32::UI::WindowsAndMessaging::*;

use super::native;
use crate::core::actions::WindowAction;

/// Why a window was refused (for diagnostics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restriction {
    None,
    Invalid,
    LoopW,
    Hidden,
    Child,
    Tool,
    Owned,
    Excluded,
    BorderlessFullscreen,
}

#[derive(Debug, Clone, Copy)]
pub struct Decision {
    pub allowed: bool,
    pub diagnostic: &'static str,
    pub restriction: Restriction,
    pub resizable: bool,
    pub borderless_fullscreen: bool,
}

fn denied(restriction: Restriction, diagnostic: &'static str) -> Decision {
    Decision {
        allowed: false,
        diagnostic,
        restriction,
        resizable: false,
        borderless_fullscreen: false,
    }
}

pub fn evaluate(hwnd: u64) -> Decision {
    let hwnd_native = native::from_raw(hwnd as isize);
    if hwnd == 0 || !native::is_window(hwnd_native) {
        return denied(
            Restriction::Invalid,
            "The target window is no longer available.",
        );
    }
    if !native::is_visible(hwnd_native) {
        return denied(Restriction::Hidden, "The target window is hidden.");
    }
    let pid = native::process_id(hwnd_native);
    if pid == 0 || pid == native::own_process_id() {
        return denied(Restriction::LoopW, "LoopW windows are not action targets.");
    }
    let style = native::window_style(hwnd_native);
    let ex_style = native::window_ex_style(hwnd_native);
    if (style as u32 & WS_CHILD.0) != 0 {
        return denied(
            Restriction::Child,
            "Child windows are not independent action targets.",
        );
    }
    if (ex_style as u32 & WS_EX_TOOLWINDOW.0) != 0 {
        return denied(Restriction::Tool, "Tool windows are not action targets.");
    }
    if !native::window_owner(hwnd_native).is_invalid() {
        return denied(
            Restriction::Owned,
            "Owned utility windows are not action targets.",
        );
    }
    if is_excluded_by_settings(pid) {
        return denied(
            Restriction::Excluded,
            "The target application is excluded in LoopW settings.",
        );
    }
    let resizable = (style as u32 & WS_THICKFRAME.0) != 0;
    let borderless = is_borderless_fullscreen(hwnd_native, style);
    Decision {
        allowed: true,
        diagnostic: "",
        restriction: if borderless {
            Restriction::BorderlessFullscreen
        } else {
            Restriction::None
        },
        resizable,
        borderless_fullscreen: borderless,
    }
}

pub fn try_authorize_action(hwnd: u64, action: WindowAction) -> Result<(), &'static str> {
    let decision = evaluate(hwnd);
    if !decision.allowed {
        super::diagnostics::report_policy(decision.diagnostic, action);
        return Err(decision.diagnostic);
    }
    if decision.borderless_fullscreen && !allows_borderless_action(action) {
        let diagnostic =
            "The target is borderless fullscreen. Exit fullscreen in the app before applying a layout.";
        super::diagnostics::report_policy(diagnostic, action);
        return Err(diagnostic);
    }
    if !decision.resizable && requires_resize(action) {
        let diagnostic = "The target window is non-resizable, so this layout action was skipped.";
        super::diagnostics::report_policy(diagnostic, action);
        return Err(diagnostic);
    }
    Ok(())
}

pub fn is_eligible_for_enumeration(hwnd: isize, excluded: u64) -> bool {
    if hwnd as u64 == excluded {
        return false;
    }
    let decision = evaluate(hwnd as u64);
    decision.allowed && !decision.borderless_fullscreen
}

pub fn is_excluded(hwnd: u64) -> bool {
    matches!(evaluate(hwnd).restriction, Restriction::Excluded)
}

/// True when the action needs a resizable target.
pub fn requires_resize(action: WindowAction) -> bool {
    use WindowAction as A;
    !matches!(
        action,
        A::NextScreen
            | A::PreviousScreen
            | A::LeftScreen
            | A::RightScreen
            | A::TopScreen
            | A::BottomScreen
            | A::MoveLeft
            | A::MoveRight
            | A::MoveUp
            | A::MoveDown
            | A::Center
            | A::Minimize
            | A::Hide
            | A::FocusUp
            | A::FocusDown
            | A::FocusLeft
            | A::FocusRight
            | A::FocusNextInStack
            | A::RestoreInitialFrame
            | A::Undo
    )
}

fn allows_borderless_action(action: WindowAction) -> bool {
    use WindowAction as A;
    matches!(
        action,
        A::Minimize
            | A::Hide
            | A::FocusUp
            | A::FocusDown
            | A::FocusLeft
            | A::FocusRight
            | A::FocusNextInStack
            | A::MinimizeOthers
            | A::RestoreInitialFrame
            | A::Undo
    )
}

fn is_excluded_by_settings(pid: u32) -> bool {
    let settings = super::shared::snapshot();
    if settings.excluded_executables.is_empty() && settings.excluded_processes.is_empty() {
        return false;
    }
    // Fast path: process-name check avoids the costlier full-path query.
    // Unicode case-insensitive compare, matching OrdinalIgnoreCase.
    if !settings.excluded_processes.is_empty() {
        let name = native::process_name(pid);
        if !name.is_empty() {
            let lower = name.to_lowercase();
            if settings
                .excluded_processes
                .iter()
                .any(|excluded| excluded.to_lowercase() == lower)
            {
                return true;
            }
        }
    }
    if !settings.excluded_executables.is_empty() {
        let path = native::executable_path(pid);
        if !path.is_empty() {
            let lower = path.to_lowercase();
            if settings
                .excluded_executables
                .iter()
                .any(|excluded| excluded.to_lowercase() == lower)
            {
                return true;
            }
        }
    }
    false
}

fn is_borderless_fullscreen(hwnd: windows::Win32::Foundation::HWND, style: isize) -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    if (style as u32 & WS_CAPTION.0) != 0 {
        return false;
    }
    let frame = unsafe {
        let mut rect = RECT::default();
        let ok = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut RECT as *mut core::ffi::c_void,
            core::mem::size_of::<RECT>() as u32,
        )
        .is_ok();
        if ok {
            native::rect_from_native(rect)
        } else {
            match native::window_rect(hwnd) {
                Some(rect) => rect,
                None => return false,
            }
        }
    };
    let monitor = match super::monitor_service::for_window(native::raw(hwnd) as u64) {
        Some(snapshot) => snapshot.monitor,
        None => return false,
    };
    (frame.left - monitor.left).abs() <= 2
        && (frame.top - monitor.top).abs() <= 2
        && (frame.right - monitor.right).abs() <= 2
        && (frame.bottom - monitor.bottom).abs() <= 2
}
