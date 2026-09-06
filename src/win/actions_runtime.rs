//! Applies `WindowAction`s to live windows (move/resize/focus/minimize).
//!
//! Pure target-frame computation lives in `crate::core::frame_math`; this
//! module owns the Win32 `SetWindowPos` / `ShowWindow` side effects behind a
//! narrow `apply` entry point used by hooks, radial, keybinds, and IPC.

use crate::core::actions::WindowAction;

/// Result of attempting an action, surfaced in status UI and IPC replies.
#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub ok: bool,
    pub message: String,
}

/// Apply an action to a window id (HWND as u64). Stubbed until Win32 wired.
pub fn apply(_window: u64, action: WindowAction) -> ApplyResult {
    ApplyResult {
        ok: false,
        message: format!("{} is unavailable until the Win32 backend is connected.", action.display_name()),
    }
}
