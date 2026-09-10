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
//! - `load_report` — clean / migrated / repaired / fallback classification
//! - `sync` — edited-vs-saved commit helper (revert on save failure)

pub mod color;
pub mod load_report;
pub mod model;
pub mod normalize;
pub mod persistence;
pub mod sync;

pub use color::normalize_color;
pub use load_report::{FallbackReason, LoadOutcome, LoadReport};
pub use model::{AppSettings, Keybind};
pub use persistence::{load, load_detailed, load_detailed_from, save, settings_path};
