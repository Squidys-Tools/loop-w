//! Window placement: undo stacks, initial frames, and the settle loop.
//!
//! Ports the `PushUndo` / `RestoreInitialFrame` / `Undo` / `PlaceWindow` /
//! `WaitForPlacement` contract, including the double-`SetWindowPlacement`
//! DPI pattern and the `SetWindowPos` fallback.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use windows::Win32::UI::WindowsAndMessaging::*;

use super::native;
use crate::core::rect::Rect;

const MAX_UNDO_DEPTH: usize = 20;
const PLACE_TOLERANCE: i32 = 2;

static UNDO_STACKS: OnceLock<Mutex<HashMap<u64, Vec<WINDOWPLACEMENT>>>> = OnceLock::new();
static INITIAL_FRAMES: OnceLock<Mutex<HashMap<u64, Rect>>> = OnceLock::new();

fn undo_stacks() -> &'static Mutex<HashMap<u64, Vec<WINDOWPLACEMENT>>> {
    UNDO_STACKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn initial_frames() -> &'static Mutex<HashMap<u64, Rect>> {
    INITIAL_FRAMES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn rects_equal(first: Rect, second: Rect) -> bool {
    (first.left - second.left).abs() <= PLACE_TOLERANCE
        && (first.top - second.top).abs() <= PLACE_TOLERANCE
        && (first.width() - second.width()).abs() <= PLACE_TOLERANCE
        && (first.height() - second.height()).abs() <= PLACE_TOLERANCE
}

/// Record the current placement for undo + first-touch initial frame.
pub fn push_undo(hwnd: u64) {
    let native_hwnd = native::from_raw(hwnd as isize);
    let Some(placement) = native::window_placement(native_hwnd) else {
        return;
    };
    if let Ok(mut frames) = initial_frames().lock() {
        frames
            .entry(hwnd)
            .or_insert_with(|| native::rect_from_native(placement.rcNormalPosition));
    }
    if let Ok(mut stacks) = undo_stacks().lock() {
        let stack = stacks.entry(hwnd).or_default();
        stack.push(placement);
        if stack.len() > MAX_UNDO_DEPTH {
            stack.drain(0..stack.len() - MAX_UNDO_DEPTH);
        }
    }
}

pub fn restore_initial_frame(hwnd: u64) -> Result<String, String> {
    let frame = initial_frames()
        .lock()
        .ok()
        .and_then(|frames| frames.get(&hwnd).copied());
    let Some(frame) = frame else {
        return Err("No original frame has been recorded for this window.".to_string());
    };
    push_undo(hwnd);
    if !place_window(hwnd, frame) {
        return Err(
            "Windows rejected the restore. The target may be elevated, protected, or non-resizable."
                .to_string(),
        );
    }
    Ok("Restored the window's original frame".to_string())
}

pub fn undo(hwnd: u64) -> Result<String, String> {
    let placement = undo_stacks()
        .lock()
        .ok()
        .and_then(|mut stacks| stacks.get_mut(&hwnd).and_then(|stack| stack.pop()));
    let Some(placement) = placement else {
        return Err("No previous placement to undo.".to_string());
    };
    if !native::set_placement(native::from_raw(hwnd as isize), &placement) {
        return Err("Could not restore the previous placement.".to_string());
    }
    Ok("Restored the previous placement".to_string())
}

/// Move a window to `frame`, settling maximized/minimized state first.
pub fn place_window(hwnd: u64, frame: Rect) -> bool {
    let native_hwnd = native::from_raw(hwnd as isize);
    let Some(mut current) = native::window_placement(native_hwnd) else {
        return false;
    };
    // Give a minimized window a moment to come back (up to 10 x 50 ms).
    for _ in 0..10 {
        if current.showCmd != SW_SHOWMINIMIZED.0 as u32 {
            break;
        }
        std::thread::sleep(core::time::Duration::from_millis(50));
        match native::window_placement(native_hwnd) {
            Some(updated) => current = updated,
            None => return false,
        }
    }
    let mut next = WINDOWPLACEMENT::default();
    next.length = core::mem::size_of::<WINDOWPLACEMENT>() as u32;
    next.showCmd = SW_RESTORE.0 as u32;
    next.flags = WPF_ASYNCWINDOWPLACEMENT;
    next.rcNormalPosition = native::rect_to_native(frame);
    // Double-SetWindowPlacement: the FancyZones DPI-reliability pattern.
    if !native::set_placement(native_hwnd, &next) {
        return false;
    }
    let _ = native::set_placement(native_hwnd, &next);
    if wait_for_placement(hwnd, frame) {
        return true;
    }
    // Fallback for apps that ignore placement.
    native::show_window(native_hwnd, SW_RESTORE);
    if !native::set_pos(native_hwnd, frame) {
        return false;
    }
    wait_for_placement(hwnd, frame)
}

fn wait_for_placement(hwnd: u64, frame: Rect) -> bool {
    let native_hwnd = native::from_raw(hwnd as isize);
    let deadline = native::tick_count().saturating_add(600);
    let mut previous: Option<Rect> = None;
    while native::tick_count() < deadline {
        let Some(placement) = native::window_placement(native_hwnd) else {
            return false;
        };
        let maximized = placement.showCmd == SW_SHOWMAXIMIZED.0 as u32
            || native::is_zoomed(native_hwnd);
        if !maximized {
            if let Some(actual) = native::window_rect(native_hwnd) {
                if rects_equal(actual, frame) {
                    return true;
                }
                if previous == Some(actual) {
                    // Stabilized elsewhere (app-clamped): accept.
                    return true;
                }
                previous = Some(actual);
            }
        }
        std::thread::sleep(core::time::Duration::from_millis(40));
    }
    !native::is_zoomed(native_hwnd)
}
