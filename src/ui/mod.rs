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

/// Boot the resident daemon (hidden start, tray-first lifecycle).
pub fn run_daemon(
    startup_command: Option<String>,
    instance: crate::win::instance::InstanceGuard,
) -> iced::Result {
    app::run(startup_command, instance)
}
