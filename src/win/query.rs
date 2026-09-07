//! Live window enumeration + focus-candidate queries.
//!
//! Ports `WindowQuery`: `EnumWindows` in z-order, iconic/zero-area filtering,
//! policy eligibility, and the focusable/minimizable/layout projections.

use windows::core::BOOL;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::native;
use super::policy;
use crate::core::rect::Rect;

/// A visible top-level window candidate in z-order.
#[derive(Debug, Clone)]
pub struct WindowCandidate {
    pub hwnd: u64,
    pub frame: Rect,
}

/// All eligible windows in z-order, excluding `excluded` (raw HWND, 0 = none).
pub fn enumerate(excluded: u64) -> Vec<WindowCandidate> {
    enumerate_excluding(excluded)
}

/// Enumerate with an excluded window. The LPARAM packing keeps the callback
/// signature-compatible while carrying both the exclusion and the output.
pub fn enumerate_excluding(excluded: u64) -> Vec<WindowCandidate> {
    struct Pack {
        excluded: u64,
        out: Vec<WindowCandidate>,
    }
    unsafe extern "system" fn packed_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let pack = &mut *(lparam.0 as *mut Pack);
        let raw = native::raw(hwnd);
        if raw as u64 == pack.excluded {
            return BOOL::from(true);
        }
        if native::is_iconic(hwnd) {
            return BOOL::from(true);
        }
        if !policy::is_eligible_for_enumeration(raw, pack.excluded) {
            return BOOL::from(true);
        }
        if let Some(frame) = native::window_rect(hwnd) {
            if frame.width() > 0 && frame.height() > 0 {
                pack.out.push(WindowCandidate {
                    hwnd: raw as u64,
                    frame,
                });
            }
        }
        BOOL::from(true)
    }
    let mut pack = Pack {
        excluded,
        out: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(Some(packed_proc), LPARAM(&mut pack as *mut Pack as isize));
    }
    pack.out
}

/// Best-effort foreground window id. Returns 0 when unavailable.
pub fn foreground_window() -> u64 {
    native::raw(native::foreground_window()) as u64
}

/// Eligible for snap start: not minimized + policy-eligible.
pub fn is_eligible_for_snap(hwnd: u64) -> bool {
    let native_hwnd = native::from_raw(hwnd as isize);
    !native::is_iconic(native_hwnd)
        && policy::is_eligible_for_enumeration(native::raw(native_hwnd), 0)
}

/// Frames of other windows for FillAvailableSpace obstacles.
pub fn layout_frames(excluded: u64) -> Vec<Rect> {
    enumerate_excluding(excluded)
        .into_iter()
        .map(|candidate| candidate.frame)
        .collect()
}
