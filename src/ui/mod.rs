//! iced settings + overlay surfaces.
//!
//! The settings window is a quiet-premium, left-nav utility surface with
//! five sections (General, Radial, Preview, Appearance, Advanced). The radial
//! and preview pages lead with the same text-free canvas renderers used by
//! the live overlays — a deliberate improvement over the WPF mockups.

pub mod app;
pub mod theme;
pub mod views;
pub mod widgets;

/// Boot the iced settings application.
pub fn run_settings() -> iced::Result {
    app::run()
}
