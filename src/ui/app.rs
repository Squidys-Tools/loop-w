//! Resident daemon: tray-first lifecycle, settings + overlay windows.
//!
//! Ports `App`/`MainWindow`: hidden start with tray icon, pipe/IPC command
//! execution, trigger-driven radial + preview overlays, snap/stash driving,
//! display-change refresh, and restore-all on quit. Started with no args,
//! the app stays hidden until tray, `activate`, or a second process.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use iced::keyboard;
use iced::theme::Base;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{window, Color, Element, Length, Subscription, Task, Theme};

use crate::core::actions::WindowAction;
use crate::core::cycle::CycleState;
use crate::core::hotkey::{hotkey_name, TriggerModifierSide};
use crate::core::monitor::MonitorMoveSizePolicy;
use crate::core::radial::{angle_of, index_at, GEOMETRY};
use crate::core::radial_targets::{resolve_slot, RadialTarget, ResolvedKeybind};
use crate::core::rect::{Point, Rect};
use crate::settings::persistence;
use crate::settings::AppSettings;
use crate::win::events::{drain, RuntimeEvent};
use crate::win::{self, native};

/// Left-nav sections (General-first hierarchy from the spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Radial,
    Preview,
    Appearance,
    Advanced,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::General,
        Section::Radial,
        Section::Preview,
        Section::Appearance,
        Section::Advanced,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::General => "General",
            Section::Radial => "Radial menu",
            Section::Preview => "Preview",
            Section::Appearance => "Appearance",
            Section::Advanced => "Advanced",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Section::General => "Trigger and launch",
            Section::Radial => "Menu shape and slots",
            Section::Preview => "Target preview and snapping",
            Section::Appearance => "Theme and colors",
            Section::Advanced => "Keybinds and recovery",
        }
    }
}

/// All UI events.
#[derive(Debug, Clone)]
pub enum Message {
    Noop,
    SelectSection(Section),
    BeginTriggerCapture,
    CancelTriggerCapture,
    BeginKeybindCapture(String),
    ToggleKeybindCycle(String),
    ToggleKeybindBypass(String),
    SetLaunchAtLogin(bool),
    NudgeDelay(i32),
    NudgeTimeout(i32),
    SetDoubleClick(bool),
    SetMiddleClick(bool),
    SetRadialEnabled(bool),
    SetCursorInteraction(bool),
    SetOuterRadius(f32),
    SetInnerRadius(f32),
    ClearWedge(usize),
    SetPreviewEnabled(bool),
    SetDragSnap(bool),
    SetPreviewPadding(f32),
    SetPreviewRadius(f32),
    SetPreviewBorder(f32),
    SetSnapThreshold(f32),
    SetRestoreOnCancel(bool),
    SetAppearanceMode(String),
    ApplyPreset(String),
    EditAccent(String),
    EditSectorFill(String),
    EditSectorStroke(String),
    EditRingFill(String),
    EditPreviewBorderColor(String),
    AddKeybind,
    DeleteKeybind(String),
    SetKeybindAction(String, String),
    SetMonitorPolicy(String),
    NudgeGlobalPadding(i32),
    EditExcludedProcesses(String),
    ResetSection(Section),
    ConfirmResetAll,
    CancelResetAll,
    ResetAll,
    // Resident-runtime messages.
    FrameTick,
    MainOpened(window::Id),
    RadialOpened(window::Id),
    PreviewOpened(window::Id),
    WindowClosed(window::Id),
    OverlayKey(keyboard::Key),
    OverlayCommit,
    RunStartupCommand,
}

/// Open radial overlay session. Geometry is physical pixels (cursor,
/// hover math, HWND matching); `window_*` fields are the logical values
/// handed to iced for sizing/positioning on scaled displays.
struct RadialSession {
    id: window::Id,
    target_hwnd: u64,
    center: Point,
    outer: f64,
    inner: f64,
    hovered: Option<usize>,
    targets: Vec<RadialTarget>,
    center_target: RadialTarget,
    patch_tries: u8,
    window_size: (f32, f32),
    /// DPI scale at open: converts physical geometry to logical for iced.
    scale: f64,
}

/// Open preview overlay session (`frame` physical, `window_size` logical).
struct PreviewSession {
    id: window::Id,
    frame: Rect,
    patch_tries: u8,
    window_size: (f32, f32),
    /// DPI scale at the frame origin: converts physical geometry to the
    /// logical values handed to iced (mirrors `RadialSession.scale`).
    scale: f64,
}

/// Mutable UI + runtime state.
pub struct State {
    pub settings: AppSettings,
    pub section: Section,
    pub status: String,
    pub color_error: Option<String>,
    pub confirming_reset_all: bool,
    pub capturing_trigger: bool,
    pub capturing_keybind: Option<String>,
    main_id: Option<window::Id>,
    radial: Option<RadialSession>,
    preview: Option<PreviewSession>,
    tray: Option<win::tray::Tray>,
    cycle: CycleState,
    // Held for process lifetime: dropping it would release the instance
    // mutex and admit a second resident.
    #[allow(dead_code)]
    instance: Option<win::instance::InstanceGuard>,
    startup_command: Option<String>,
    pending_target: Option<u64>,
}

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
    iced::daemon(State::boot, update, view)
        .title(|state: &State, id: window::Id| {
            // Overlay HWNDs are located by these stable titles when their
            // physical frame changes. Keep them distinct from the settings
            // window so preview updates do not fall back to separate iced
            // move/resize effects.
            if state.radial.as_ref().map(|session| session.id) == Some(id) {
                "LoopW Radial".to_string()
            } else if state.preview.as_ref().map(|session| session.id) == Some(id) {
                "LoopW Preview".to_string()
            } else {
                format!("LoopW Settings — {}", state.section.label())
            }
        })
        .theme(|state: &State, _window: window::Id| {
            Some(if state.settings.is_light() {
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
        .subscription(subscription)
        .run()
}

impl State {
    fn boot() -> (State, Task<Message>) {
        let settings = persistence::load();
        win::shared::init(settings.clone());
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
        let status = if !hooks_ok {
            "Could not install the global keyboard hook".to_string()
        } else if tray.is_none() {
            "Tray icon unavailable — quit via Task Manager or pipe".to_string()
        } else {
            "Saved".to_string()
        };
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
                confirming_reset_all: false,
                capturing_trigger: false,
                capturing_keybind: None,
                main_id: None,
                radial: None,
                preview: None,
                tray,
                cycle: CycleState::new(),
                instance,
                startup_command,
                pending_target: None,
            },
            task,
        )
    }
}

fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::batch([
        window::frames().map(|_| Message::FrameTick),
        // Steady pump independent of windows: with no window open (tray-only)
        // iced emits no frame events, which would starve tray polling, event
        // draining, and snap/stash tracking. Match the ~60 Hz frame cadence.
        iced::time::every(Duration::from_millis(16)).map(|_| Message::FrameTick),
        window::close_requests().map(Message::WindowClosed),
        iced::event::listen().map(|event| match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                Message::OverlayKey(key)
            }
            _ => Message::Noop,
        }),
    ])
}

// --- settings persistence (mirror -> shared -> disk -> services) ---

fn commit_settings(state: &mut State, status: &str) {
    // Reset an in-flight press only when trigger binding or behavior
    // changed (C# resets in SetBinding/SetTriggerBehavior, never in
    // SetKeybinds): dragging a slider must not cancel a held trigger.
    let trigger_changed = {
        let current = win::shared::snapshot();
        state.settings.trigger_vk != current.trigger_vk
            || state.settings.trigger_modifiers != current.trigger_modifiers
            || state.settings.trigger_modifier_side != current.trigger_modifier_side
            || state.settings.trigger_delay_ms != current.trigger_delay_ms
            || state.settings.trigger_timeout_ms != current.trigger_timeout_ms
            || state.settings.double_click_to_trigger != current.double_click_to_trigger
            || state.settings.middle_click_to_trigger != current.middle_click_to_trigger
    };
    state.settings.normalize();
    win::shared::replace(state.settings.clone());
    if win::shared::save_now() {
        state.status = status.to_string();
    } else {
        state.status = "Could not save".to_string();
    }
    if trigger_changed {
        win::hooks::notify_settings_changed();
    } else {
        win::hooks::refresh_config();
    }
    win::monitor_service::invalidate();
    win::stash_service::handle_settings_changed();
}

// --- update ---

#[allow(clippy::too_many_lines)]
fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Noop => Task::none(),
        Message::SelectSection(section) => {
            state.section = section;
            state.confirming_reset_all = false;
            Task::none()
        }
        Message::BeginTriggerCapture => {
            state.capturing_trigger = true;
            state.capturing_keybind = None;
            win::hooks::begin_capture(win::hooks::CaptureTarget::Trigger);
            state.status = "Press a key… (Esc cancels)".to_string();
            Task::none()
        }
        Message::CancelTriggerCapture => {
            state.capturing_trigger = false;
            state.capturing_keybind = None;
            win::hooks::cancel_capture();
            state.status = "Saved".to_string();
            Task::none()
        }
        Message::BeginKeybindCapture(id) => {
            state.capturing_trigger = false;
            state.capturing_keybind = Some(id.clone());
            win::hooks::begin_capture(win::hooks::CaptureTarget::Keybind(id));
            state.status = "Press a key… (Esc cancels)".to_string();
            Task::none()
        }
        Message::ToggleKeybindCycle(id) => {
            if let Some(bind) = state.settings.keybinds.iter_mut().find(|k| k.id == id) {
                bind.cycle_enabled = !bind.cycle_enabled;
            }
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::ToggleKeybindBypass(id) => {
            if let Some(bind) = state.settings.keybinds.iter_mut().find(|k| k.id == id) {
                bind.bypass_trigger = !bind.bypass_trigger;
            }
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetLaunchAtLogin(enabled) => {
            state.settings.launch_at_login = enabled;
            if win::startup::set_launch_at_login(enabled).is_err() {
                state.settings.launch_at_login = !enabled;
                state.status = "Could not update launch setting".to_string();
                return Task::none();
            }
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::NudgeDelay(delta) => {
            state.settings.trigger_delay_ms =
                (state.settings.trigger_delay_ms + delta).clamp(0, 1000);
            commit_settings(state, "Trigger updated");
            Task::none()
        }
        Message::NudgeTimeout(delta) => {
            state.settings.trigger_timeout_ms =
                (state.settings.trigger_timeout_ms + delta).clamp(0, 10_000);
            commit_settings(state, "Trigger updated");
            Task::none()
        }
        Message::SetDoubleClick(value) => {
            state.settings.double_click_to_trigger = value;
            commit_settings(state, "Trigger updated");
            Task::none()
        }
        Message::SetMiddleClick(value) => {
            state.settings.middle_click_to_trigger = value;
            commit_settings(state, "Trigger updated");
            Task::none()
        }
        Message::SetRadialEnabled(value) => {
            state.settings.radial_enabled = value;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetCursorInteraction(value) => {
            state.settings.cursor_interaction_enabled = value;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetOuterRadius(value) => {
            state.settings.radial_outer_radius = value as f64;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetInnerRadius(value) => {
            state.settings.radial_inner_radius = value as f64;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::ClearWedge(index) => {
            if let Some(slot) = state.settings.radial_slots.get_mut(index) {
                *slot = crate::core::radial_targets::RadialTargetSettings {
                    kind: crate::core::radial_targets::RadialTargetKind::None,
                    action: WindowAction::RightHalf,
                    keybind_id: String::new(),
                    cycle_enabled: false,
                };
            }
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetPreviewEnabled(value) => {
            state.settings.preview_enabled = value;
            commit_settings(state, "Saved");
            if !value {
                let mut tasks = Vec::new();
                close_preview(state, &mut tasks);
                return Task::batch(tasks);
            }
            Task::none()
        }
        Message::SetDragSnap(value) => {
            state.settings.drag_snap_enabled = value;
            commit_settings(state, "Saved");
            if !value {
                // End mid-drag as Disabled: hide preview, restore pre-drag
                // frame when a candidate was seen.
                let mut tasks = Vec::new();
                if let Some(finish) = win::snap_service::disable() {
                    apply_snap_finish(state, &mut tasks, finish);
                } else {
                    close_preview(state, &mut tasks);
                }
                return Task::batch(tasks);
            }
            Task::none()
        }
        Message::SetPreviewPadding(value) => {
            state.settings.preview_padding = value as f64;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetPreviewRadius(value) => {
            state.settings.preview_corner_radius = value as f64;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetPreviewBorder(value) => {
            state.settings.preview_border_width = value as f64;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetSnapThreshold(value) => {
            state.settings.drag_snap_threshold = value as i32;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetRestoreOnCancel(value) => {
            state.settings.restore_pre_drag_on_cancel = value;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetAppearanceMode(mode) => {
            state.settings.appearance_mode = mode;
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::ApplyPreset(name) => {
            if let Some(preset) = crate::ui::theme::PRESETS.iter().find(|p| p.name == name) {
                state.settings.accent_color = preset.accent.to_string();
                state.settings.radial_sector_fill = preset.sector_fill.to_string();
                state.settings.radial_sector_stroke = preset.sector_stroke.to_string();
                state.settings.radial_ring_fill = preset.ring_fill.to_string();
                state.settings.preview_border_color = preset.preview_border.to_string();
                state.color_error = None;
                commit_settings(state, "Saved");
            }
            Task::none()
        }
        Message::EditAccent(value) => {
            apply_color_edit(state, "accent", value);
            Task::none()
        }
        Message::EditSectorFill(value) => {
            apply_color_edit(state, "sector_fill", value);
            Task::none()
        }
        Message::EditSectorStroke(value) => {
            apply_color_edit(state, "sector_stroke", value);
            Task::none()
        }
        Message::EditRingFill(value) => {
            apply_color_edit(state, "ring_fill", value);
            Task::none()
        }
        Message::EditPreviewBorderColor(value) => {
            apply_color_edit(state, "preview_border", value);
            Task::none()
        }
        Message::AddKeybind => {
            state
                .settings
                .keybinds
                .push(crate::settings::Keybind::default());
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::DeleteKeybind(id) => {
            state.settings.keybinds.retain(|k| k.id != id);
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetKeybindAction(id, name) => {
            if let Some(bind) = state.settings.keybinds.iter_mut().find(|k| k.id == id) {
                if let Some(action) = WindowAction::ALL
                    .iter()
                    .copied()
                    .find(|action| action.display_name() == name)
                {
                    bind.action = action;
                }
            }
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::SetMonitorPolicy(policy) => {
            state.settings.monitor_move_policy = if policy == "PreserveLogicalSize" {
                MonitorMoveSizePolicy::PreserveLogicalSize
            } else {
                MonitorMoveSizePolicy::PreservePixels
            };
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::NudgeGlobalPadding(delta) => {
            state.settings.global_padding = (state.settings.global_padding + delta).clamp(0, 128);
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::EditExcludedProcesses(value) => {
            state.settings.excluded_processes = value
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            commit_settings(state, "Saved");
            Task::none()
        }
        Message::ResetSection(section) => {
            reset_section(state, section);
            commit_settings(state, "Reset complete");
            Task::none()
        }
        Message::ConfirmResetAll => {
            state.confirming_reset_all = true;
            Task::none()
        }
        Message::CancelResetAll => {
            state.confirming_reset_all = false;
            Task::none()
        }
        Message::ResetAll => {
            state.settings.reset_all();
            state.color_error = None;
            state.confirming_reset_all = false;
            commit_settings(state, "Reset complete");
            Task::none()
        }
        Message::FrameTick => frame_tick(state),
        Message::MainOpened(id) => {
            state.main_id = Some(id);
            Task::none()
        }
        Message::RadialOpened(id) => {
            if let Some(session) = state.radial.as_mut() {
                session.id = id;
            }
            Task::none()
        }
        Message::PreviewOpened(id) => {
            if let Some(session) = state.preview.as_mut() {
                session.id = id;
            }
            Task::none()
        }
        Message::WindowClosed(id) => {
            if state.main_id == Some(id) {
                // The settings window opts out of iced's automatic close so
                // the daemon can remain resident. Explicitly close the
                // native window here; clearing only the id leaves its last
                // painted surface behind as a black/ghost window.
                state.main_id = None;
                return window::close(id);
            }
            if state.radial.as_ref().map(|s| s.id) == Some(id) {
                state.radial = None;
                let mut tasks = Vec::new();
                close_preview(state, &mut tasks);
                state.status = "Cancelled".to_string();
                return Task::batch(tasks);
            }
            if state.preview.as_ref().map(|s| s.id) == Some(id) {
                state.preview = None;
            }
            Task::none()
        }
        Message::OverlayKey(key) => {
            let mut tasks = Vec::new();
            overlay_key_pressed(state, &mut tasks, key);
            Task::batch(tasks)
        }
        Message::OverlayCommit => {
            let mut tasks = Vec::new();
            commit_radial(state, &mut tasks);
            Task::batch(tasks)
        }
        Message::RunStartupCommand => {
            if let Some(command) = state.startup_command.take() {
                let mut tasks = Vec::new();
                let reply = execute_command(state, &mut tasks, &command);
                native::print_cli_line(&reply);
                return Task::batch(tasks);
            }
            Task::none()
        }
    }
}

/// Reject a captured keybind that collides with the trigger or duplicates
/// another row (C# HasKeybindConflict): the colliding bind would be
/// permanently inert (trigger-Vk exclusion) or shadowed (first-wins).
fn keybind_conflict(
    settings: &AppSettings,
    skip_id: &str,
    modifiers: u32,
    vk: u32,
) -> Option<String> {
    use crate::core::hotkey::hotkey_name;
    use crate::core::hotkey::TriggerModifierSide;
    let label = hotkey_name(modifiers, vk, TriggerModifierSide::Any);
    if modifiers == settings.trigger_modifiers && vk == settings.trigger_vk {
        return Some(format!("{label} is already the trigger."));
    }
    if settings
        .keybinds
        .iter()
        .any(|other| other.id != skip_id && other.modifiers == modifiers && other.vk == vk)
    {
        return Some(format!("{label} is already used by another keybind."));
    }
    None
}

fn apply_color_edit(state: &mut State, field: &str, value: String) {
    let normalized = crate::settings::normalize_color(&value, "__invalid__");
    if normalized == "__invalid__" {
        state.color_error = Some(format!("{field}: use #RRGGBB or #AARRGGBB."));
        return;
    }
    state.color_error = None;
    match field {
        "accent" => state.settings.accent_color = value,
        "sector_fill" => state.settings.radial_sector_fill = value,
        "sector_stroke" => state.settings.radial_sector_stroke = value,
        "ring_fill" => state.settings.radial_ring_fill = value,
        "preview_border" => state.settings.preview_border_color = value,
        _ => {}
    }
    commit_settings(state, "Saved");
}

fn reset_section(state: &mut State, section: Section) {
    let defaults = AppSettings::default();
    match section {
        Section::General => {
            state.settings.trigger_vk = defaults.trigger_vk;
            state.settings.trigger_modifiers = defaults.trigger_modifiers;
            state.settings.trigger_modifier_side = TriggerModifierSide::Any;
            state.settings.trigger_delay_ms = defaults.trigger_delay_ms;
            state.settings.trigger_timeout_ms = defaults.trigger_timeout_ms;
            state.settings.double_click_to_trigger = defaults.double_click_to_trigger;
            state.settings.middle_click_to_trigger = defaults.middle_click_to_trigger;
            state.settings.launch_at_login = defaults.launch_at_login;
        }
        Section::Radial => {
            state.settings.radial_enabled = defaults.radial_enabled;
            state.settings.cursor_interaction_enabled = defaults.cursor_interaction_enabled;
            state.settings.radial_outer_radius = defaults.radial_outer_radius;
            state.settings.radial_inner_radius = defaults.radial_inner_radius;
            state.settings.radial_slots = defaults.radial_slots;
            state.settings.center_target = defaults.center_target;
        }
        Section::Preview => {
            state.settings.preview_enabled = defaults.preview_enabled;
            state.settings.preview_padding = defaults.preview_padding;
            state.settings.preview_corner_radius = defaults.preview_corner_radius;
            state.settings.preview_border_width = defaults.preview_border_width;
            state.settings.preview_border_color = defaults.preview_border_color;
            state.settings.drag_snap_enabled = defaults.drag_snap_enabled;
            state.settings.drag_snap_threshold = defaults.drag_snap_threshold;
            state.settings.restore_pre_drag_on_cancel = defaults.restore_pre_drag_on_cancel;
        }
        Section::Appearance => {
            state.settings.appearance_mode = defaults.appearance_mode;
            state.settings.accent_color = defaults.accent_color;
            state.settings.radial_sector_fill = defaults.radial_sector_fill;
            state.settings.radial_sector_stroke = defaults.radial_sector_stroke;
            state.settings.radial_ring_fill = defaults.radial_ring_fill;
            state.settings.preview_border_color = defaults.preview_border_color;
            state.color_error = None;
        }
        Section::Advanced => {
            state.settings.keybinds = defaults.keybinds;
            state.settings.monitor_move_policy = defaults.monitor_move_policy;
            state.settings.global_padding = defaults.global_padding;
            state.settings.padding_left = defaults.padding_left;
            state.settings.padding_top = defaults.padding_top;
            state.settings.padding_right = defaults.padding_right;
            state.settings.padding_bottom = defaults.padding_bottom;
            state.settings.excluded_executables = defaults.excluded_executables;
            state.settings.excluded_processes = defaults.excluded_processes;
            state.settings.stash_peek = defaults.stash_peek;
            state.settings.stash_hit_zone = defaults.stash_hit_zone;
            state.settings.stash_reveal_delay_ms = defaults.stash_reveal_delay_ms;
        }
    }
}

// --- resident runtime ---

/// Once-per-frame pump: events, tray, stash, snap, overlay hover, patching.
fn frame_tick(state: &mut State) -> Task<Message> {
    let mut tasks: Vec<Task<Message>> = Vec::new();
    if let Some(tray) = state.tray.as_ref() {
        win::tray::poll(tray);
    }
    // Stash records change under us (persist/prune paths): mirror them.
    state.settings.stash_records = win::shared::snapshot().stash_records;
    for event in drain() {
        handle_runtime(state, &mut tasks, event);
    }
    if let Some(cursor) = native::cursor_pos() {
        // Snap tracking (preview honors the PreviewEnabled gate here so a
        // disabled preview never opens for drags; hover path gates inside).
        let (track, finish) = win::snap_service::track(cursor);
        match track {
            win::snap_service::SnapTrack::Show(frame)
            | win::snap_service::SnapTrack::Update(frame) => {
                if state.settings.preview_enabled {
                    ensure_preview(state, &mut tasks, frame);
                }
            }
            win::snap_service::SnapTrack::Hide => close_preview(state, &mut tasks),
            win::snap_service::SnapTrack::Idle => {}
        }
        if let Some(finish) = finish {
            apply_snap_finish(state, &mut tasks, finish);
        }
        // Stash hover polling (internally 80 ms throttled).
        if let Some(message) = win::stash_service::poll(cursor) {
            state.status = message;
        }
        // Radial hover follows the cursor.
        update_radial_hover(state, &mut tasks, cursor);
    }
    // Overlay style patching retries (windows appear a tick after open).
    if let Some(session) = state.radial.as_mut() {
        if session.patch_tries < 12 && win::overlay::patch_tool_window(radial_expected(session)) {
            session.patch_tries = 12;
        } else if session.patch_tries < 12 {
            session.patch_tries += 1;
        }
    }
    if let Some(session) = state.preview.as_mut() {
        if session.patch_tries < 12 && win::overlay::patch_click_through(session.frame) {
            session.patch_tries = 12;
        } else if session.patch_tries < 12 {
            session.patch_tries += 1;
        }
    }
    // The preview opens and moves after the radial, so it settles above it
    // in the topmost band and hides the menu where they overlap. Pin the
    // preview back below the radial (no activation, no focus steal); this
    // retries through the async open/move window like the patching above.
    if let (Some(radial), Some(preview)) = (state.radial.as_ref(), state.preview.as_ref()) {
        win::overlay::order_preview_below_radial(preview.frame, radial_expected(radial));
    }
    Task::batch(tasks)
}

#[allow(clippy::too_many_lines)]
fn handle_runtime(state: &mut State, tasks: &mut Vec<Task<Message>>, event: RuntimeEvent) {
    match event {
        RuntimeEvent::TriggerPressed => {
            let foreground = win::query::foreground_window();
            let pid = if foreground == 0 {
                0
            } else {
                native::process_id(native::from_raw(foreground as isize))
            };
            if foreground == 0 || pid == 0 || pid == native::own_process_id() {
                state.status = format!(
                    "Focus another app, then hold {}",
                    hotkey_name(
                        state.settings.trigger_modifiers,
                        state.settings.trigger_vk,
                        state.settings.trigger_modifier_side,
                    )
                );
                state.pending_target = None;
            } else if state.radial.is_some() {
                // Already open (e.g. chorded re-press): never leak the prior id.
                state.pending_target = Some(foreground);
            } else {
                state.pending_target = Some(foreground);
                if state.settings.radial_enabled {
                    open_radial(state, tasks, foreground);
                }
            }
        }
        RuntimeEvent::TriggerReleased => {
            state.pending_target = None;
            if state.radial.is_some() {
                commit_radial(state, tasks);
            }
        }
        RuntimeEvent::TriggerTimedOut => {
            state.pending_target = None;
            close_overlays(state, tasks);
            state.status = "Trigger timed out — no action committed".to_string();
        }
        RuntimeEvent::TriggerCancelled => {
            state.pending_target = None;
            close_overlays(state, tasks);
            state.status = "Trigger cancelled".to_string();
        }
        RuntimeEvent::KeybindFired {
            action,
            cycle_enabled,
            bypass_trigger,
        } => {
            // Resolve the target BEFORE dismissing: a failed keybind must
            // leave a live overlay alone (C# only dismisses on success path).
            let target = if bypass_trigger {
                let foreground = win::query::foreground_window();
                let pid = if foreground == 0 {
                    0
                } else {
                    native::process_id(native::from_raw(foreground as isize))
                };
                if foreground == 0 || pid == 0 || pid == native::own_process_id() {
                    state.status = "No external foreground window is available.".to_string();
                    return;
                }
                foreground
            } else {
                match state.pending_target {
                    Some(target) => target,
                    None => {
                        state.status = format!(
                            "  ·  Capture a target first with {}",
                            hotkey_name(
                                state.settings.trigger_modifiers,
                                state.settings.trigger_vk,
                                state.settings.trigger_modifier_side,
                            )
                        );
                        return;
                    }
                }
            };
            // Dismiss any overlay first so a later release cannot double-commit.
            close_overlays(state, tasks);
            state.status = commit_action(state, target, action, cycle_enabled);
        }
        RuntimeEvent::RevealStashed => {
            close_overlays(state, tasks);
            state.status = win::stash_service::reveal_next().unwrap_or_else(|error| error);
        }
        RuntimeEvent::CaptureUpdate {
            modifiers,
            vk,
            keybind,
        } => match keybind {
            Some(id) => {
                if let Some(reason) = keybind_conflict(&state.settings, &id, modifiers, vk) {
                    state.capturing_keybind = None;
                    state.capturing_trigger = false;
                    state.status = reason;
                    return;
                }
                if let Some(bind) = state.settings.keybinds.iter_mut().find(|k| k.id == id) {
                    bind.modifiers = modifiers;
                    bind.vk = vk;
                }
                state.capturing_keybind = None;
                state.capturing_trigger = false;
                commit_settings(state, "Keybind updated");
            }
            None => {
                state.settings.trigger_modifiers = modifiers;
                state.settings.trigger_vk = vk;
                state.capturing_trigger = false;
                state.capturing_keybind = None;
                commit_settings(state, "Trigger updated");
            }
        },
        RuntimeEvent::CaptureCancelled => {
            state.capturing_trigger = false;
            state.capturing_keybind = None;
            state.status = "Rebind cancelled".to_string();
        }
        RuntimeEvent::CaptureRejected => {
            state.capturing_trigger = false;
            state.capturing_keybind = None;
            state.status = "That key is reserved by Windows. Choose another.".to_string();
        }
        RuntimeEvent::PipeCommand { id, command } => {
            let reply = execute_command(state, tasks, &command);
            win::ipc::set_reply(id, reply);
        }
        RuntimeEvent::ShowSettings | RuntimeEvent::TrayShowSettings => {
            show_main(state, tasks);
        }
        RuntimeEvent::TrayQuit => {
            // C# OnExit order: pipe stop -> tray dispose -> stash restore ->
            // window close -> hook dispose. Windows themselves are reclaimed
            // at process teardown; the tray icon MUST drop first so it never
            // lingers, and hooks go last so no event fires mid-teardown.
            win::ipc::stop_server();
            state.tray = None;
            win::stash_service::restore_all();
            win::hooks::stop();
            tasks.push(iced::exit());
        }
        RuntimeEvent::DisplaysChanged => {
            win::monitor_service::invalidate();
            // Stale preview must never linger; hover reopens it next tick.
            close_preview(state, tasks);
        }
        RuntimeEvent::SnapBegin { cursor, .. } => {
            win::snap_service::begin_at_cursor(cursor);
        }
        RuntimeEvent::SnapEnd { released } => {
            let _ = released;
            // Only a live drag session may touch the preview: a bare click
            // release must never close a radial hover-preview.
            if !win::snap_service::is_active() {
                return;
            }
            if let Some(finish) = win::snap_service::end_released() {
                apply_snap_finish(state, tasks, finish);
            } else {
                close_preview(state, tasks);
            }
        }
    }
}

fn commit_action(
    state: &mut State,
    hwnd: u64,
    action: WindowAction,
    cycle_enabled: bool,
) -> String {
    let selection = state.cycle.select(hwnd, action, cycle_enabled);
    match win::actions_runtime::apply(hwnd, selection.effective) {
        Ok(message) => {
            state.cycle.commit(hwnd, selection);
            format!("{}{}", message, selection.status_suffix())
        }
        Err(error) => error,
    }
}

fn apply_snap_finish(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    finish: win::snap_service::SnapFinish,
) {
    close_preview(state, tasks);
    state.status = match finish {
        win::snap_service::SnapFinish::Apply {
            window,
            action,
            frame,
        } => win::actions_runtime::apply_snap(window, action, frame).unwrap_or_else(|error| error),
        win::snap_service::SnapFinish::Restore { window, frame } => {
            win::actions_runtime::restore_frame(window, frame).unwrap_or_else(|error| error)
        }
    };
}

// --- overlays ---

fn open_radial(state: &mut State, tasks: &mut Vec<Task<Message>>, target_hwnd: u64) {
    let Some(cursor) = native::cursor_pos() else {
        return;
    };
    let settings = &state.settings;
    let outer = settings.radial_outer_radius;
    let inner = settings.radial_inner_radius;
    let keybinds: Vec<ResolvedKeybind> = settings
        .keybinds
        .iter()
        .map(|bind| ResolvedKeybind {
            id: bind.id.clone(),
            action: bind.action,
            label: hotkey_name(bind.modifiers, bind.vk, TriggerModifierSide::Any),
        })
        .collect();
    let targets = settings
        .radial_slots
        .iter()
        .map(|slot| resolve_slot(slot, &keybinds))
        .collect();
    let center_target = resolve_slot(&settings.center_target, &keybinds);
    let bounds = win::overlay::radial_bounds(cursor, outer);
    // iced positions/sizes in logical pixels: convert once at open.
    let scale = native::dpi_scale_at_point(cursor);
    let (lx, ly, lw, lh) = win::overlay::to_logical(bounds, scale);
    let (id, task) = window::open(radial_window_settings(lx, ly, lw, lh));
    state.radial = Some(RadialSession {
        id,
        target_hwnd,
        center: cursor,
        outer,
        inner,
        hovered: None,
        targets,
        center_target,
        patch_tries: 0,
        window_size: (lw, lh),
        scale,
    });
    tasks.push(task.map(Message::RadialOpened));
}

fn radial_expected(session: &RadialSession) -> Rect {
    win::overlay::radial_bounds(session.center, session.outer)
}

fn wedge_at(session: &RadialSession, cursor: Point) -> Option<usize> {
    if !state_cursor_interaction() {
        return session.hovered;
    }
    let dx = cursor.x as f64 - session.center.x as f64;
    let dy = cursor.y as f64 - session.center.y as f64;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < session.inner || distance > session.outer + 12.0 {
        return None;
    }
    Some(index_at(angle_of(dx, dy)))
}

fn state_cursor_interaction() -> bool {
    win::shared::snapshot().cursor_interaction_enabled
}

fn update_radial_hover(state: &mut State, tasks: &mut Vec<Task<Message>>, cursor: Point) {
    match state.radial.as_mut() {
        Some(session) => {
            session.hovered = wedge_at(session, cursor);
        }
        None => return,
    }
    // Always refresh while open: display/DPI/work-area changes must move a
    // live preview even when the cursor (and hover) did not.
    refresh_preview_for_hover(state, tasks);
}

fn refresh_preview_for_hover(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if !state.settings.preview_enabled {
        close_preview(state, tasks);
        return;
    }
    let frame = state.radial.as_ref().and_then(|session| {
        let target = session
            .hovered
            .and_then(|index| session.targets.get(index))?;
        let action = target.action()?;
        win::target_frame::target_frame(session.target_hwnd, action).ok()
    });
    match frame {
        Some(frame) => ensure_preview(state, tasks, frame),
        None => close_preview(state, tasks),
    }
}

fn ensure_preview(state: &mut State, tasks: &mut Vec<Task<Message>>, frame: Rect) {
    if frame.width() <= 0 || frame.height() <= 0 {
        return;
    }
    if let Some(session) = state.preview.as_ref() {
        if session.frame == frame {
            return;
        }
        let id = session.id;
        // Preview frames are physical; the window lives in logical pixels.
        let scale = native::dpi_scale_at_point(Point::new(frame.left, frame.top));
        let (lx, ly, lw, lh) = win::overlay::to_logical(frame, scale);
        if let Some(session) = state.preview.as_mut() {
            session.frame = frame;
            session.window_size = (lw, lh);
            session.scale = scale;
        }
        // Once the window exists, keep its native frame in physical pixels.
        // Sending move and resize as separate logical iced effects can race
        // during a quadrant/half transition: the compositor may paint the
        // old canvas into the new, smaller/larger surface for a frame and
        // leave one section clipped. SetWindowPos updates both dimensions
        // atomically in the same coordinate space used by target_frame.
        if !win::overlay::set_preview_frame(frame) {
            // The HWND can briefly be unavailable while an open/close is
            // settling. Keep the logical fallback for that short window.
            tasks.push(window::move_to(id, iced::Point::new(lx, ly)));
            tasks.push(window::resize(id, iced::Size::new(lw, lh)));
        }
        return;
    }
    let scale = native::dpi_scale_at_point(Point::new(frame.left, frame.top));
    let (lx, ly, lw, lh) = win::overlay::to_logical(frame, scale);
    let (id, task) = window::open(preview_window_settings(lx, ly, lw, lh));
    state.preview = Some(PreviewSession {
        id,
        frame,
        patch_tries: 0,
        window_size: (lw, lh),
        scale,
    });
    tasks.push(task.map(Message::PreviewOpened));
}

fn close_preview(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if let Some(session) = state.preview.take() {
        tasks.push(window::close(session.id));
    }
}

fn close_overlays(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if let Some(session) = state.radial.take() {
        tasks.push(window::close(session.id));
    }
    close_preview(state, tasks);
}

fn commit_radial(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    let Some(session) = state.radial.take() else {
        return;
    };
    tasks.push(window::close(session.id));
    close_preview(state, tasks);
    let target = session
        .hovered
        .and_then(|index| session.targets.get(index).cloned());
    match target {
        Some(RadialTarget::BuiltIn { action, cycle })
        | Some(RadialTarget::Keybind { action, cycle, .. }) => {
            state.status = commit_action(state, session.target_hwnd, action, cycle);
            return;
        }
        _ => {}
    }
    // No wedge hovered: release inside the center hole commits the configured
    // center action; anywhere else cancels without touching the window.
    if let Some(action) = session.center_target.action() {
        if let Some(cursor) = native::cursor_pos() {
            let dx = cursor.x as f64 - session.center.x as f64;
            let dy = cursor.y as f64 - session.center.y as f64;
            if dx * dx + dy * dy < session.inner * session.inner {
                let cycle = session.center_target.cycle_enabled();
                state.status = commit_action(state, session.target_hwnd, action, cycle);
                return;
            }
        }
    }
    state.status = "Cancelled".to_string();
}

fn overlay_key_pressed(state: &mut State, tasks: &mut Vec<Task<Message>>, key: keyboard::Key) {
    use iced::keyboard::key::Named;
    if state.radial.is_none() {
        return;
    }
    match key {
        keyboard::Key::Named(Named::Escape) => {
            close_overlays(state, tasks);
            state.status = "Cancelled".to_string();
        }
        keyboard::Key::Named(Named::Enter) => commit_radial(state, tasks),
        keyboard::Key::Named(Named::ArrowLeft) => select_wedge(state, tasks, 4),
        keyboard::Key::Named(Named::ArrowRight) => select_wedge(state, tasks, 0),
        keyboard::Key::Named(Named::ArrowUp) => select_wedge(state, tasks, 6),
        keyboard::Key::Named(Named::ArrowDown) => select_wedge(state, tasks, 2),
        _ => {}
    }
}

fn select_wedge(state: &mut State, tasks: &mut Vec<Task<Message>>, index: usize) {
    if index >= GEOMETRY.len() {
        return;
    }
    if let Some(session) = state.radial.as_mut() {
        session.hovered = Some(index);
    }
    refresh_preview_for_hover(state, tasks);
}

// --- command execution (IPC + startup command) ---

fn execute_command(state: &mut State, tasks: &mut Vec<Task<Message>>, command: &str) -> String {
    // A panicking handler must reply, never crash the resident loop.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        execute_command_inner(state, tasks, command)
    }));
    result.unwrap_or_else(|_| "ERROR: command execution failed.".to_string())
}

#[allow(clippy::too_many_lines)]
fn execute_command_inner(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    command: &str,
) -> String {
    match crate::core::commands::parse_command(command) {
        Err(error) => format!("ERROR: {error}"),
        Ok(cmd) => match cmd {
            crate::core::commands::LoopCommand::Activate => {
                show_main(state, tasks);
                "LoopW activated.".to_string()
            }
            crate::core::commands::LoopCommand::Apply(action) => {
                if action == WindowAction::RevealStashed {
                    return win::stash_service::reveal_next().unwrap_or_else(|error| error);
                }
                let foreground = win::query::foreground_window();
                let pid = if foreground == 0 {
                    0
                } else {
                    native::process_id(native::from_raw(foreground as isize))
                };
                if foreground == 0 || pid == 0 || pid == native::own_process_id() {
                    return "No external foreground window is available.".to_string();
                }
                match win::actions_runtime::apply(foreground, action) {
                    Ok(message) => message,
                    Err(error) => error,
                }
            }
            crate::core::commands::LoopCommand::ListActions => {
                crate::core::commands::format_actions()
            }
            crate::core::commands::LoopCommand::ListKeybinds => {
                let settings = win::shared::snapshot();
                crate::core::commands::format_keybinds(
                    &settings
                        .keybinds
                        .iter()
                        .map(|bind| {
                            (
                                bind.modifiers,
                                bind.vk,
                                bind.action,
                                bind.cycle_enabled,
                                bind.bypass_trigger,
                            )
                        })
                        .collect::<Vec<_>>(),
                    settings.trigger_modifiers,
                    settings.trigger_vk,
                    settings.trigger_modifier_side,
                )
            }
            crate::core::commands::LoopCommand::ListAll => {
                format!(
                    "Actions:\r\n{}\r\n\r\nKeybinds:\r\n{}",
                    crate::core::commands::format_actions(),
                    {
                        let settings = win::shared::snapshot();
                        crate::core::commands::format_keybinds(
                            &settings
                                .keybinds
                                .iter()
                                .map(|bind| {
                                    (
                                        bind.modifiers,
                                        bind.vk,
                                        bind.action,
                                        bind.cycle_enabled,
                                        bind.bypass_trigger,
                                    )
                                })
                                .collect::<Vec<_>>(),
                            settings.trigger_modifiers,
                            settings.trigger_vk,
                            settings.trigger_modifier_side,
                        )
                    }
                )
            }
        },
    }
}

// --- windows ---

fn main_window_settings() -> window::Settings {
    window::Settings {
        size: iced::Size::new(1166.0, 779.0),
        min_size: Some(iced::Size::new(1100.0, 760.0)),
        position: window::Position::Centered,
        resizable: true,
        decorations: true,
        transparent: false,
        level: window::Level::Normal,
        exit_on_close_request: false,
        ..Default::default()
    }
}

fn radial_window_settings(x: f32, y: f32, w: f32, h: f32) -> window::Settings {
    window::Settings {
        size: iced::Size::new(w, h),
        position: window::Position::Specific(iced::Point::new(x, y)),
        resizable: false,
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        ..Default::default()
    }
}

fn preview_window_settings(x: f32, y: f32, w: f32, h: f32) -> window::Settings {
    window::Settings {
        size: iced::Size::new(w, h),
        position: window::Position::Specific(iced::Point::new(x, y)),
        resizable: false,
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        ..Default::default()
    }
}

fn show_main(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    match state.main_id {
        Some(id) => tasks.push(window::gain_focus(id)),
        None => {
            let (id, task) = window::open(main_window_settings());
            state.main_id = Some(id);
            tasks.push(task.map(Message::MainOpened));
        }
    }
}

// --- view ---

fn view(state: &State, window: window::Id) -> Element<'_, Message> {
    if Some(window) == state.main_id {
        return settings_view(state);
    }
    if let Some(session) = &state.radial {
        if session.id == window {
            let mut canvas =
                crate::ui::widgets::radial_canvas::RadialCanvas::from_settings(&state.settings);
            // Canvas shares the window's logical space, not physical pixels.
            let scale = session.scale as f32;
            canvas.outer_radius = session.outer as f32 / scale;
            canvas.inner_radius = session.inner as f32 / scale;
            canvas.hovered = session.hovered;
            return super::views::overlay::radial(canvas, session.window_size.0);
        }
    }
    if let Some(session) = &state.preview {
        if session.id == window {
            let mut canvas =
                crate::ui::widgets::preview_canvas::PreviewCanvas::from_settings(&state.settings);
            // Canvas shares the window's logical space, not physical pixels
            // (same scaling as the radial overlay above): without this the
            // padding, corner radius, and border grow with the DPI scale.
            let scale = session.scale as f32;
            if scale > 0.0 && scale.is_finite() {
                canvas.padding /= scale;
                canvas.corner_radius /= scale;
                canvas.border_width /= scale;
            }
            return super::views::overlay::preview(
                canvas,
                session.window_size.0,
                session.window_size.1,
            );
        }
    }
    container(text("")).into()
}

fn settings_view(state: &State) -> Element<'_, Message> {
    let sidebar = Section::ALL.iter().fold(
        column![text("LoopW").size(18), text("Settings").size(12)].spacing(4),
        |col, section| {
            let active = *section == state.section;
            let label = if active {
                format!("● {}", section.label())
            } else {
                format!("○ {}", section.label())
            };
            col.push(
                button(text(label).size(14))
                    .on_press(Message::SelectSection(*section))
                    .width(Length::Fill),
            )
            .push(text(section.hint()).size(11))
        },
    );

    let content: Element<'_, Message> = match state.section {
        Section::General => super::views::general::view(state),
        Section::Radial => super::views::radial::view(state),
        Section::Preview => super::views::preview::view(state),
        Section::Appearance => super::views::appearance::view(state),
        Section::Advanced => super::views::advanced::view(state),
    };

    let layout = row![
        container(sidebar.spacing(6).padding(16)).width(Length::Fixed(240.0)),
        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill),
    ];

    container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        // Opaque settings background, painted explicitly: the daemon-wide
        // clear color is transparent (for the overlay windows), so this
        // opaque window must fill its own surface. Same color the clear
        // used to provide (`Base::base`), so the look is unchanged.
        .style(|theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(Base::base(theme).background_color)),
            ..Default::default()
        })
        .into()
}
