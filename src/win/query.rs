//! Window query helpers (foreground window, enumeration, focus).
//!
//! Live implementation uses the `windows` crate; pure neighbor-selection
//! math is tested in `crate::core` via the services below.

use crate::core::rect::Rect;

/// A visible top-level window candidate.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: u64,
    pub frame: Rect,
    pub title: String,
}

/// Best-effort foreground window id. Returns 0 when unavailable.
pub fn foreground_window() -> u64 {
    #[cfg(windows)]
    {
        // Live GetForegroundWindow wiring lands with the Win32 backend.
        0
    }
    #[cfg(not(windows))]
    {
        0
    }
}
