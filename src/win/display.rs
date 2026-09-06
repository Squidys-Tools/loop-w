//! Display/DPI change watcher: hidden broadcast window.
//!
//! Ports the `WindowMessageHook` display branch: on WM_DISPLAYCHANGE,
//! WM_DPICHANGED, WM_SETTINGCHANGE, or WM_DEVICECHANGE the monitor cache
//! is invalidated, snap targets re-resolve, and stale previews hide —
//! without aborting any in-flight gesture.

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

use super::events::{RuntimeEvent, push};

const CLASS_NAME: &str = "LoopWDisplayWatcher";

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE | WM_DEVICECHANGE => {
            push(RuntimeEvent::DisplaysChanged);
            LRESULT(0)
        }
        _ => DefWindowProcW(Some(hwnd), message, wparam, lparam),
    }
}

/// Spawn the watcher thread (idempotent). No window is ever shown.
pub fn start() {
    static STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    STARTED.get_or_init(|| {
        std::thread::Builder::new()
            .name("loopw-display".to_string())
            .spawn(|| unsafe {
                let class = WNDCLASSW {
                    lpfnWndProc: Some(wnd_proc),
                    hInstance: HINSTANCE::default(),
                    lpszClassName: w!(CLASS_NAME),
                    ..WNDCLASSW::default()
                };
                RegisterClassW(&class);
                let hwnd = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!(CLASS_NAME),
                    w!("LoopW display watcher"),
                    WINDOW_STYLE::default(),
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    None,
                    None,
                );
                if let Ok(hwnd) = hwnd {
                    let mut message = MSG::default();
                    while GetMessageW(&mut message, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                    let _ = DestroyWindow(hwnd);
                }
            })
            .expect("display watcher spawns");
    });
}
