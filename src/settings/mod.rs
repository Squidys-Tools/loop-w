//! Settings persistence model, normalization, and atomic save/load.
//!
//! The JSON contract intentionally matches the C# `AppSettings` keys so
//! existing `%LOCALAPPDATA%\LoopW\settings.json` files load without loss.
//! Unknown fields are ignored; missing fields fall back to safe defaults.
//!
//! Layout (each file < 300 lines):
//! - `model` — `AppSettings`, `Keybind`, defaults, reset helpers
//! - `color` — `#RRGGBB` / `#AARRGGBB` normalization
//! - `normalize` — clamping and list repair
//! - `persistence` — settings path, atomic load/save

pub mod color;
pub mod model;
pub mod normalize;
pub mod persistence;

pub use color::normalize_color;
pub use model::{AppSettings, Keybind};
pub use persistence::{load, save, settings_path};
