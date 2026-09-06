//! LoopW — radial window manager (Rust + iced port).
//!
//! Module layout:
//! - `core` — pure math, settings model, commands (fully tested)
//! - `settings` — persistence + normalization (same JSON contract as C#)
//! - `win` — Windows runtime: hooks, window actions, snap, stash, IPC, tray
//! - `ui` — iced settings + radial/preview surfaces (better Fluent-style UX)

pub mod core;
pub mod settings;
pub mod ui;
pub mod win;

mod cli;

fn main() -> iced::Result {
    cli::run()
}
