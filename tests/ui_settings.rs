//! Public-contract coverage for the iced settings surface.
//!
//! The package is currently a binary-only crate, so this test harness includes
//! the production modules instead of importing a library target. Keeping the
//! assertions at the public API boundary is intentional: `app::update` and
//! the resident `State` constructor are private and also own Win32 side
//! effects. Full message-transition coverage needs a small test seam in
//! `src/ui/app.rs`; the exact seam is reported with this workstream.

#![allow(dead_code, unused_imports)]
#![allow(clippy::field_reassign_with_default)]

#[path = "../src/core/mod.rs"]
mod core;
#[path = "../src/settings/mod.rs"]
mod settings;

#[path = "../src/ui/widgets/preview_canvas.rs"]
pub mod preview_canvas_impl;
#[path = "../src/ui/widgets/radial_canvas.rs"]
pub mod radial_canvas_impl;
#[path = "../src/ui/theme.rs"]
pub mod theme_impl;

mod ui {
    pub use crate::theme_impl as theme;

    pub mod widgets {
        pub use crate::preview_canvas_impl as preview_canvas;
        pub use crate::radial_canvas_impl as radial_canvas;
    }
}

use core::hotkey::TriggerModifierSide;
use settings::sync::{self, SAVE_ERROR_MESSAGE};
use settings::AppSettings;
use ui::theme::matching_preset;
use ui::widgets::preview_canvas::PreviewCanvas;
use ui::widgets::radial_canvas::RadialCanvas;

#[test]
fn canvas_models_follow_persisted_settings() {
    let mut settings = AppSettings::default();
    settings.radial_outer_radius = 118.0;
    settings.radial_inner_radius = 42.0;
    settings.preview_padding = 30.0;
    settings.preview_corner_radius = 19.0;
    settings.preview_border_width = 3.5;

    let radial = RadialCanvas::from_settings(&settings);
    assert_eq!(radial.outer_radius, 118.0);
    assert_eq!(radial.inner_radius, 42.0);

    let preview = PreviewCanvas::from_settings(&settings);
    assert_eq!(preview.padding, 30.0);
    assert_eq!(preview.corner_radius, 19.0);
    assert_eq!(preview.border_width, 3.5);
    assert_eq!(preview.target, None);
}

#[test]
fn reset_all_restores_every_settings_field_to_factory_defaults() {
    let defaults = AppSettings::default();
    let mut settings = defaults.clone();
    settings.trigger_vk = 0x41;
    settings.trigger_modifier_side = TriggerModifierSide::Left;
    settings.keybinds.push(Default::default());
    settings.radial_outer_radius = 140.0;
    settings.preview_enabled = false;
    settings.global_padding = 24;
    settings.excluded_processes = vec!["game.exe".to_string()];
    settings.accent_color = "#112233".to_string();

    settings.reset_all();

    assert_eq!(settings, defaults);
}

#[test]
fn commit_normalizes_candidate_before_the_persist_callback() {
    let saved = AppSettings::default();
    let mut candidate = saved.clone();
    candidate.trigger_delay_ms = -50;
    candidate.trigger_timeout_ms = 99_999;
    candidate.radial_outer_radius = 1.0;
    candidate.accent_color = "not-a-color".to_string();

    let mut persisted = None;
    let report = sync::commit(
        candidate,
        saved,
        |to_save| {
            persisted = Some(to_save.clone());
            true
        },
        "Saved",
    );

    assert!(report.succeeded);
    let persisted = persisted.expect("commit should call persist on success");
    assert_eq!(persisted, report.settings);
    assert_eq!(persisted.trigger_delay_ms, 0);
    assert_eq!(persisted.trigger_timeout_ms, 10_000);
    assert_eq!(persisted.radial_outer_radius, 64.0);
    assert_eq!(persisted.accent_color, "#FF007AFF");
}

#[test]
fn failed_commit_reverts_the_ui_candidate_and_sets_sticky_error() {
    let saved = AppSettings::default();
    let mut candidate = saved.clone();
    candidate.preview_enabled = false;

    let report = sync::commit(candidate, saved.clone(), |_| false, "Saved");

    assert!(!report.succeeded);
    assert_eq!(report.settings, saved);
    assert_eq!(report.status, SAVE_ERROR_MESSAGE);
    assert_eq!(report.save_error.as_deref(), Some(SAVE_ERROR_MESSAGE));
    assert!(!report.trigger_changed);
}

#[test]
fn matching_preset_tracks_the_appearance_page_state() {
    let preset = &ui::theme::PRESETS[1];
    assert_eq!(
        matching_preset(
            preset.accent,
            preset.sector_fill,
            preset.sector_stroke,
            preset.ring_fill,
            preset.preview_border,
        ),
        Some(preset.name)
    );

    assert_eq!(
        matching_preset(
            preset.accent,
            preset.sector_fill,
            preset.sector_stroke,
            preset.ring_fill,
            "#010203",
        ),
        None
    );
}
