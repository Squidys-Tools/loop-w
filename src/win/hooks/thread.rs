//! Hook thread lifecycle: install WH_KEYBOARD_LL + WH_MOUSE_LL, pump the
//! message loop LL delivery requires, remove both on `WM_QUIT_LOOP`.
//!
//! The hook procs only lock the shared state and run `state_machine`
//! reducers — swallow decisions and queued `RuntimeEvent`s are decided
//! there, never app logic inline.

use std::time::Duration;

use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::consts::{
    ACTIVATION_TIMER_ID, TIMEOUT_TIMER_ID, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_QUIT_LOOP, WM_SYSKEYDOWN,
    WM_SYSKEYUP, WM_TIMER,
};
use super::{handles, state, state_machine};
use crate::win::{diagnostics, snap_service};

/// Install hooks on a dedicated thread. Returns true when active.
pub fn start() -> bool {
    let Ok(mut guards) = handles().lock() else {
        return false;
    };
    if guards.running {
        return !guards.keyboard.is_invalid();
    }
    guards.running = true;
    drop(guards);
    std::thread::Builder::new()
        .name("loopw-hooks".to_string())
        .spawn(hook_thread)
        .expect("hook thread spawns");
    // Give the thread a moment to install before reporting status.
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(10));
        if let Ok(guards) = handles().lock() {
            if !guards.keyboard.is_invalid() || !guards.running {
                break;
            }
        }
    }
    is_active()
}

pub fn is_active() -> bool {
    handles()
        .lock()
        .map(|guards| !guards.keyboard.is_invalid())
        .unwrap_or(false)
}

/// Remove hooks and stop the pump thread.
pub fn stop() {
    let thread_id = handles().lock().map(|g| g.thread_id).unwrap_or(0);
    if thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT_LOOP, WPARAM(0), LPARAM(0));
        }
    }
}

fn hook_thread() {
    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code < 0 {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        let message = wparam.0 as u32;
        let is_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
        let is_up = message == WM_KEYUP || message == WM_SYSKEYUP;
        if !is_down && !is_up {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        let vk = (*(lparam.0 as *const KBDLLHOOKSTRUCT)).vkCode;
        let swallow = state()
            .lock()
            .map(|mut guard| {
                if is_down {
                    state_machine::handle_key_down_locked(&mut guard, vk)
                } else {
                    state_machine::handle_key_up_locked(&mut guard, vk)
                }
            })
            .unwrap_or(false);
        if swallow {
            LRESULT(1)
        } else {
            CallNextHookEx(None, code, wparam, lparam)
        }
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code < 0 {
            return CallNextHookEx(None, code, wparam, lparam);
        }
        let message = wparam.0 as u32;
        let swallow = match message {
            WM_MBUTTONDOWN | WM_MBUTTONUP => state().lock().map(|mut guard| {
                if message == WM_MBUTTONDOWN {
                    state_machine::handle_middle_down_locked(&mut guard)
                } else {
                    state_machine::handle_middle_up_locked(&mut guard)
                }
            }),
            _ => Ok(false),
        };
        // Title-bar drag traffic goes to the snap tracker (never swallowed);
        // middle-button traffic drives the alternate trigger only.
        match message {
            WM_LBUTTONDOWN => snap_service::note_button(true),
            WM_LBUTTONUP => snap_service::note_button(false),
            _ => {}
        }
        if swallow.unwrap_or(false) {
            LRESULT(1)
        } else {
            CallNextHookEx(None, code, wparam, lparam)
        }
    }

    unsafe {
        let keyboard = SetWindowsHookExW(
            WINDOWS_HOOK_ID(WH_KEYBOARD_LL),
            Some(keyboard_proc),
            None,
            0,
        )
        .unwrap_or_default();
        let mouse = SetWindowsHookExW(WINDOWS_HOOK_ID(WH_MOUSE_LL), Some(mouse_proc), None, 0)
            .unwrap_or_default();
        if let Ok(mut guards) = handles().lock() {
            guards.keyboard = keyboard;
            guards.mouse = mouse;
            guards.thread_id = GetCurrentThreadId();
        }
        if keyboard.is_invalid() {
            diagnostics::report_hook_install("SetWindowsHookExW failed for WH_KEYBOARD_LL");
        } else if mouse.is_invalid() {
            diagnostics::report_hook_install(
                "SetWindowsHookExW failed for WH_MOUSE_LL; middle-click trigger unavailable",
            );
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            if message.message == WM_TIMER {
                state_machine::handle_timer(message.wParam.0);
                continue;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = KillTimer(None, ACTIVATION_TIMER_ID);
        let _ = KillTimer(None, TIMEOUT_TIMER_ID);
        if let Ok(guards) = handles().lock() {
            if !guards.keyboard.is_invalid() {
                let _ = UnhookWindowsHookEx(guards.keyboard);
            }
            if !guards.mouse.is_invalid() {
                let _ = UnhookWindowsHookEx(guards.mouse);
            }
        }
        if let Ok(mut guards) = handles().lock() {
            guards.keyboard = HHOOK::default();
            guards.mouse = HHOOK::default();
            guards.thread_id = 0;
            guards.running = false;
        }
        if let Ok(mut guard) = state().lock() {
            state_machine::reset_input_locked(&mut guard, false);
        }
    }
}
