//! Hook thread: installs the low-level procs, pumps the message loop
//! required for LL delivery, and tears everything down on quit.

use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::machine::{
    handle_key_down_locked, handle_key_up_locked, handle_middle_down_locked,
    handle_middle_up_locked, handle_timer, reset_input_locked,
};
use super::{
    handles, state, ACTIVATION_TIMER_ID, TIMEOUT_TIMER_ID, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN,
    WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_SYSKEYDOWN,
    WM_SYSKEYUP, WM_TIMER,
};

pub(super) fn hook_thread() {
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
                    handle_key_down_locked(&mut guard, vk)
                } else {
                    handle_key_up_locked(&mut guard, vk)
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
                    handle_middle_down_locked(&mut guard)
                } else {
                    handle_middle_up_locked(&mut guard)
                }
            }),
            _ => Ok(false),
        };
        // Title-bar drag traffic goes to the snap tracker (never swallowed);
        // middle-button traffic drives the alternate trigger only.
        match message {
            WM_LBUTTONDOWN => crate::win::snap_service::note_button(true),
            WM_LBUTTONUP => crate::win::snap_service::note_button(false),
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
            crate::win::diagnostics::report_hook_install(
                "SetWindowsHookExW failed for WH_KEYBOARD_LL",
            );
        } else if mouse.is_invalid() {
            crate::win::diagnostics::report_hook_install(
                "SetWindowsHookExW failed for WH_MOUSE_LL; middle-click trigger unavailable",
            );
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            if message.message == WM_TIMER {
                handle_timer(message.wParam.0);
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
            reset_input_locked(&mut guard, false);
        }
    }
}
