//! Resident daemon: tray-first lifecycle, settings + overlay windows.
//!
//! Ports `App`/`MainWindow`: hidden start with tray icon, pipe/IPC command
//! execution, trigger-driven radial + preview overlays, snap/stash driving,
//! display-change refresh, and restore-all on quit. Started with no args,
//! the app stays hidden until tray, `activate`, or a second process.

mod message;
mod overlays;
mod update;
mod view;

// `state` stays ui-visible: `State` fields in the `pub(in crate::ui)` band
// expose `RadialSession`/`PreviewSession`, so the type path must reach the
// same scope (siblings touch them only through field access, never by name).
pub(in crate::ui) mod state;

use iced::theme::Base;
use iced::{window, Color, Theme};
use std::sync::{Mutex, OnceLock};

use crate::win;

pub use message::{Message, Section};
pub(super) use overlays::{
    close_overlays, close_preview, commit_radial, ensure_preview, open_radial, open_tray_menu,
    radial_expected, update_radial_hover,
};
pub use state::State;

struct BootArgs {
    startup: Option<String>,
    instance: Option<win::instance::InstanceGuard>,
}

static BOOT_ARGS: OnceLock<Mutex<Option<BootArgs>>> = OnceLock::new();

/// Boot the resident daemon. `instance` is held for process lifetime.
pub fn run(
    startup_command: Option<String>,
    instance: win::instance::InstanceGuard,
) -> iced::Result {
    *BOOT_ARGS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("boot args poisoned") = Some(BootArgs {
        startup: startup_command,
        instance: Some(instance),
    });
    iced::daemon(State::boot, update::update, view::view)
        .title(|state: &State, id: window::Id| {
            // Overlay HWNDs are located by these stable titles when their
            // physical frame changes. Keep them distinct from the settings
            // window so preview updates do not fall back to separate iced
            // move/resize effects.
            if state.radial.as_ref().map(|session| session.id) == Some(id) {
                "LoopW Radial".to_string()
            } else if state.preview.as_ref().map(|session| session.id) == Some(id) {
                "LoopW Preview".to_string()
            } else if state.tray_menu == Some(id) {
                "LoopW Tray Menu".to_string()
            } else {
                format!("LoopW Settings — {}", state.section.label())
            }
        })
        .theme(|state: &State, window| {
            Some(if state.tray_menu == Some(window) {
                Theme::Dark
            } else if state.settings.is_light() {
                Theme::Light
            } else {
                Theme::Dark
            })
        })
        // Transparent clear for every window: iced clears each surface
        // with this color, and `transparent: true` on the overlay windows
        // only affects compositing — without this the radial/preview
        // overlays render an opaque theme-background square. The opaque
        // settings window paints its own background (see `settings_view`).
        .style(|_state: &State, theme: &Theme| iced::theme::Style {
            background_color: Color::TRANSPARENT,
            ..Base::base(theme)
        })
        .subscription(super::runtime::subscription)
        .run()
}

#[cfg(test)]
mod tests {
    use super::state::test_state;
    use super::Section;
    use crate::core::actions::WindowAction;
    use crate::core::radial_targets::RadialTargetKind;
    use crate::settings::AppSettings;

    #[test]
    fn motion_preference_resolves_against_the_windows_flag() {
        use crate::core::motion::MotionPreference;

        let mut state = test_state();
        state.windows_animations_enabled = false;

        // Follow Windows mirrors the OS flag.
        state.settings.reduce_motion = MotionPreference::FollowWindows;
        assert!(crate::ui::runtime::reduced_motion(&state));

        // An explicit choice wins in both directions.
        state.settings.reduce_motion = MotionPreference::FullMotion;
        assert!(!crate::ui::runtime::reduced_motion(&state));
        state.windows_animations_enabled = true;
        assert!(!crate::ui::runtime::reduced_motion(&state));
        state.settings.reduce_motion = MotionPreference::Reduced;
        assert!(crate::ui::runtime::reduced_motion(&state));
    }

    #[test]
    fn radial_target_choices_round_trip_built_in_and_none() {
        let settings = AppSettings::default();

        let none = crate::ui::settings::radial_target_from_choice(&settings, "No action");
        assert_eq!(none.kind, RadialTargetKind::None);

        let action = crate::ui::settings::radial_target_from_choice(&settings, "Action: Left half");
        assert_eq!(action.kind, RadialTargetKind::Action);
        assert_eq!(action.action, WindowAction::LeftHalf);
    }

    #[test]
    fn radial_target_choice_resolves_stable_keybind_id() {
        let mut settings = AppSettings::default();
        let bind = crate::settings::Keybind::default();
        let id = bind.id.clone();
        let action = bind.action;
        settings.keybinds.push(bind);

        let target = crate::ui::settings::radial_target_from_choice(
            &settings,
            &format!("Keybind: {id} (Space)"),
        );
        assert_eq!(target.kind, RadialTargetKind::Keybind);
        assert_eq!(target.keybind_id, id);
        assert_eq!(target.action, action);
    }

    #[test]
    fn radial_settings_view_builds_the_assignment_surface() {
        let mut state = test_state();
        state.section = Section::Radial;
        let _ = crate::ui::views::radial::view(&state);
    }
}
