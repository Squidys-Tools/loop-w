#![windows_subsystem = "windows"]

//! LoopW — radial window manager (Rust + iced port).
//!
//! Module layout:
//! - `core` — pure math, settings model, commands (fully tested)
//! - `settings` — persistence + normalization (same JSON contract as C#)
//! - `win` — Windows runtime: hooks, window actions, snap, stash, IPC, tray
//! - `ui` — resident iced daemon: settings + radial/preview overlays

pub mod core;
pub mod settings;
pub mod ui;
pub mod win;

mod cli;

fn main() -> iced::Result {
    // Per-monitor V2 awareness so every rect stays in physical pixels.
    // (Replaces the C# app.manifest; falls back gracefully on Win7.)
    enable_dpi_awareness();
    cli::run()
}

#[cfg(windows)]
fn enable_dpi_awareness() {
    use windows::Win32::UI::HiDpi::*;
    use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).is_ok() {
            return;
        }
        let _ = SetProcessDPIAware();
    }
}

#[cfg(not(windows))]
fn enable_dpi_awareness() {}
