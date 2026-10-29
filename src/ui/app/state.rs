use iced::{window, Task};

use crate::core::cycle::CycleState;
use crate::core::radial_targets::RadialTarget;
use crate::core::rect::{Point, Rect};
use crate::settings::persistence;
use crate::settings::AppSettings;
use crate::win::{self, native};

use super::{Message, Section, BOOT_ARGS};

/// Open radial overlay session. Geometry is physical pixels (cursor,
/// hover math, HWND matching); `window_*` fields are the logical values
/// handed to iced for sizing/positioning on scaled displays.
pub struct RadialSession {
    pub id: window::Id,
    pub target_hwnd: u64,
    pub center: Point,
    pub outer: f64,
    pub inner: f64,
    pub hovered: Option<usize>,
    pub targets: Vec<RadialTarget>,
    pub center_target: RadialTarget,
    pub patch_tries: u8,
    pub window_size: (f32, f32),
    /// DPI scale at open: converts physical geometry to logical for iced.
    pub scale: f64,
    pub(super) preview_cache: Vec<Option<PreviewFrameCache>>,
}

pub(super) struct PreviewFrameCache {
    pub(super) generation: u64,
    pub(super) result: Result<Rect, String>,
}

/// Open preview overlay session.
///
/// `frame` is the target rectangle in physical screen pixels. The preview
/// HWND is deliberately kept at `overlay_frame` for the lifetime of the
/// session; only the target rectangle drawn inside it changes on hover.
pub struct PreviewSession {
    pub id: window::Id,
    pub frame: Rect,
    pub overlay_frame: Rect,
    pub patch_tries: u8,
    pub window_size: (f32, f32),
    /// DPI scale at the frame origin: converts physical geometry to the
    /// logical values handed to iced (mirrors `RadialSession.scale`).
    pub scale: f64,
    /// Number of z-order repair attempts after the asynchronous windows open.
    /// Once the native windows are found and ordered, repeating the Win32
    /// lookup on every runtime tick only adds work and cannot improve the UI.
    pub order_tries: u8,
}

/// Mutable UI + runtime state.
pub struct State {
    pub settings: AppSettings,
    /// Live Windows animation flag, sampled at boot and whenever the settings
    /// surface opens or the preference changes — never per frame, so a view
    /// build never issues a syscall and the 16 ms overlay cadence never polls
    /// it 60 times a second.
    pub(in crate::ui) windows_animations_enabled: bool,
    pub section: Section,
    pub status: String,
    pub color_error: Option<String>,
    /// Sticky save failure (cleared on the next successful save). The same
    /// message is also copied into `status` so every existing section view
    /// shows it without a new diagnostics surface.
    pub save_error: Option<String>,
    pub confirming_reset_all: bool,
    pub capturing_trigger: bool,
    pub capturing_keybind: Option<String>,
    pub(in crate::ui) main_id: Option<window::Id>,
    pub(in crate::ui) radial: Option<RadialSession>,
    pub(in crate::ui) preview: Option<PreviewSession>,
    pub(in crate::ui) tray_menu: Option<window::Id>,
    pub(in crate::ui) tray_menu_patch_tries: u8,
    pub(in crate::ui) tray: Option<win::tray::Tray>,
    pub(in crate::ui) cycle: CycleState,
    // Held for process lifetime: dropping it would release the instance
    // mutex and admit a second resident.
    #[allow(dead_code)]
    pub(in crate::ui) instance: Option<win::instance::InstanceGuard>,
    pub(in crate::ui) startup_command: Option<String>,
    pub(in crate::ui) pending_target: Option<u64>,
    /// Revision observed when the shared settings mirror was last synchronized.
    pub(in crate::ui) settings_revision: u64,
}

impl State {
    pub(super) fn boot() -> (State, Task<Message>) {
        // `load_detailed` keeps the tolerant fallback but reports *why*, so a
        // corrupt or migrated file is visible instead of a silent "Saved".
        let report = persistence::load_detailed();
        win::shared::init(report.settings.clone());
        let settings = report.settings;
        let tray = win::tray::build().ok();
        win::hooks::start();
        let hooks_ok = win::hooks::is_active();
        win::display::start();
        win::ipc::start_server();
        win::stash_service::restore_persisted();
        let args = BOOT_ARGS
            .get()
            .and_then(|slot| slot.lock().ok())
            .and_then(|mut slot| slot.take());
        let (startup_command, instance) = match args {
            Some(args) => (args.startup, args.instance),
            None => (None, None),
        };
        let mut status = if !hooks_ok {
            "Could not install the global keyboard hook".to_string()
        } else if tray.is_none() {
            "Tray icon unavailable — quit via Task Manager or pipe".to_string()
        } else {
            "Saved".to_string()
        };
        // Surface load problems without hiding a hook/tray failure behind
        // them: the load message wins only when the runtime itself is fine.
        let mut save_error: Option<String> = None;
        if report.outcome.is_fallback()
            && !matches!(
                report.outcome,
                crate::settings::LoadOutcome::Fallback(crate::settings::FallbackReason::Missing)
            )
        {
            let message = format!(
                "Settings file {} — restored defaults.",
                report.outcome.describe()
            );
            save_error = Some(message.clone());
            if hooks_ok && tray.is_some() {
                status = message;
            }
        }
        let task = if startup_command.is_some() {
            Task::done(Message::RunStartupCommand)
        } else {
            Task::none()
        };
        (
            State {
                settings,
                section: Section::General,
                status,
                color_error: None,
                save_error,
                windows_animations_enabled: native::animations_enabled(),
                confirming_reset_all: false,
                capturing_trigger: false,
                capturing_keybind: None,
                main_id: None,
                radial: None,
                preview: None,
                tray_menu: None,
                tray_menu_patch_tries: 0,
                tray,
                cycle: CycleState::new(),
                instance,
                startup_command,
                pending_target: None,
                settings_revision: win::shared::settings_revision(),
            },
            task,
        )
    }
}

#[cfg(test)]
pub(super) fn test_state() -> State {
    State {
        settings: AppSettings::default(),
        section: Section::General,
        status: String::new(),
        color_error: None,
        save_error: None,
        windows_animations_enabled: true,
        confirming_reset_all: false,
        capturing_trigger: false,
        capturing_keybind: None,
        main_id: None,
        radial: None,
        preview: None,
        tray_menu: None,
        tray_menu_patch_tries: 0,
        tray: None,
        cycle: CycleState::new(),
        instance: None,
        startup_command: None,
        pending_target: None,
        settings_revision: 0,
    }
}
