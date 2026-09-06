//! System-tray resident lifecycle (last priority per plan).
//!
//! The app starts hidden; Settings shows the iced window, Close hides back
//! to tray, Quit removes hooks and exits. Live icon wiring lands here after
//! radial, preview, and settings are verified.

/// Tray actions surfaced to the iced runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ShowSettings,
    Quit,
}
