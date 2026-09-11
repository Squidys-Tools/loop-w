//! Resident daemon: tray-first lifecycle, settings + overlay windows.
//!
//! Ports `App`/`MainWindow`: hidden start with tray icon, pipe/IPC command
//! execution, trigger-driven radial + preview overlays, snap/stash driving,
//! display-change refresh, and restore-all on quit. Started with no args,
//! the app stays hidden until tray, `activate`, or a second process.

use iced::keyboard;
use iced::theme::Base;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{window, Color, Element, Length, Rectangle, Task, Theme};
use std::sync::{Mutex, OnceLock};

use crate::core::cycle::CycleState;
use crate::core::hotkey::{hotkey_name, TriggerModifierSide};
use crate::core::radial::{index_at_vector, GEOMETRY};
use crate::core::radial_targets::{resolve_slot, RadialTarget, ResolvedKeybind};
use crate::core::rect::{Point, Rect};
use crate::settings::persistence;
use crate::settings::AppSettings;
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
    SelectSection(Section),
    BeginTriggerCapture,
    CancelTriggerCapture,
    BeginKeybindCapture(String),
    SetModifierSide(String),
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
    SetRadialTarget(Option<usize>, String),
    ToggleRadialCycle(Option<usize>),
    ClearWedge(usize),
    SetPreviewEnabled(bool),
    SetDragSnap(bool),
    SetPreviewPadding(f32),
    SetPreviewRadius(f32),
    SetPreviewBorder(f32),
    SetSnapThreshold(f32),
    SetRestoreOnCancel(bool),
    SetStashPersistence(bool),
    NudgeStashPeek(i32),
    NudgeStashHitZone(i32),
    NudgeStashDelay(i32),
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
    NudgePaddingLeft(i32),
    NudgePaddingTop(i32),
    NudgePaddingRight(i32),
    NudgePaddingBottom(i32),
    EditExcludedExecutables(String),
    EditExcludedProcesses(String),
    ResetSection(Section),
    ConfirmResetAll,
    CancelResetAll,
    ResetAll,
    ClearDiagnostics,
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
pub(super) struct RadialSession {
    pub(super) id: window::Id,
    pub(super) target_hwnd: u64,
    pub(super) center: Point,
    pub(super) outer: f64,
    pub(super) inner: f64,
    pub(super) hovered: Option<usize>,
    pub(super) targets: Vec<RadialTarget>,
    pub(super) center_target: RadialTarget,
    pub(super) patch_tries: u8,
    pub(super) window_size: (f32, f32),
    /// DPI scale at open: converts physical geometry to logical for iced.
    pub(super) scale: f64,
}

/// Open preview overlay session.
///
/// `frame` is the target rectangle in physical screen pixels. The preview
/// HWND is deliberately kept at `overlay_frame` for the lifetime of the
/// session; only the target rectangle drawn inside it changes on hover.
pub(super) struct PreviewSession {
    pub(super) id: window::Id,
    pub(super) frame: Rect,
    pub(super) overlay_frame: Rect,
    pub(super) patch_tries: u8,
    pub(super) window_size: (f32, f32),
    /// DPI scale at the frame origin: converts physical geometry to the
    /// logical values handed to iced (mirrors `RadialSession.scale`).
    pub(super) scale: f64,
    /// Number of z-order repair attempts after the asynchronous windows open.
    /// Once the native windows are found and ordered, repeating the Win32
    /// lookup on every runtime tick only adds work and cannot improve the UI.
    pub(super) order_tries: u8,
}

/// Mutable UI + runtime state.
pub struct State {
    pub settings: AppSettings,
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
    pub(super) main_id: Option<window::Id>,
    pub(super) radial: Option<RadialSession>,
    pub(super) preview: Option<PreviewSession>,
    pub(super) tray: Option<win::tray::Tray>,
    pub(super) cycle: CycleState,
    // Held for process lifetime: dropping it would release the instance
    // mutex and admit a second resident.
    #[allow(dead_code)]
    pub(super) instance: Option<win::instance::InstanceGuard>,
    pub(super) startup_command: Option<String>,
    pub(super) pending_target: Option<u64>,
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
        .subscription(super::runtime::subscription)
        .run()
}

impl State {
    fn boot() -> (State, Task<Message>) {
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

// --- update ---

#[allow(clippy::too_many_lines)]
fn update(state: &mut State, message: Message) -> Task<Message> {
    let message = match super::settings::try_update(state, message) {
        Ok(task) => return task,
        Err(message) => message,
    };
    match message {
        Message::FrameTick => super::runtime::frame_tick(state),
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
            let mut tasks = Vec::new();
            super::runtime::run_startup_command(state, &mut tasks);
            Task::batch(tasks)
        }
        _ => unreachable!("settings message was not handled before runtime dispatch"),
    }
}

// --- overlays ---

pub(super) fn open_radial(state: &mut State, tasks: &mut Vec<Task<Message>>, target_hwnd: u64) {
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
    let (id, task) = window::open(super::windows::radial_settings(lx, ly, lw, lh));
    if let Some(preview) = state.preview.as_mut() {
        // A radial can be opened while a drag preview is already alive. That
        // creates a new z-order relationship which needs one settling pass.
        preview.order_tries = 0;
    }
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

pub(super) fn radial_expected(session: &RadialSession) -> Rect {
    win::overlay::radial_bounds(session.center, session.outer)
}

fn wedge_at(session: &RadialSession, cursor: Point) -> Option<usize> {
    if !state_cursor_interaction() {
        return session.hovered;
    }
    wedge_index_at(session.center, session.inner, cursor)
}

/// Resolve a radial selection from the ray through the menu center.
///
/// The menu's outer edge is visual only: once the cursor has left the dead
/// zone, its distance from the center must not change which wedge is active.
fn wedge_index_at(center: Point, inner: f64, cursor: Point) -> Option<usize> {
    let dx = cursor.x as f64 - center.x as f64;
    let dy = cursor.y as f64 - center.y as f64;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < inner {
        return None;
    }
    Some(index_at_vector(dx, dy))
}

fn state_cursor_interaction() -> bool {
    win::shared::cursor_interaction_enabled()
}

pub(super) fn update_radial_hover(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    cursor: Point,
) {
    let changed = match state.radial.as_mut() {
        Some(session) => {
            let hovered = wedge_at(session, cursor);
            let changed = session.hovered != hovered;
            session.hovered = hovered;
            changed
        }
        None => return,
    };
    // Target-frame resolution crosses into Win32 and can be relatively
    // expensive. Re-resolve only when the ray selects a different wedge, or
    // when an external event (display/settings change) left the preview gone.
    if changed || state.preview.is_none() {
        refresh_preview_for_hover(state, tasks);
    }
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

pub(super) fn ensure_preview(state: &mut State, tasks: &mut Vec<Task<Message>>, frame: Rect) {
    if frame.width() <= 0 || frame.height() <= 0 {
        return;
    }
    let overlay_frame = preview_overlay_frame(frame);
    if let Some(session) = state.preview.as_ref() {
        if session.frame == frame {
            return;
        }
        // Most hover transitions stay on one monitor. Keep the HWND and its
        // iced surface stable in that case. Re-submit its unchanged size as
        // a redraw boundary: iced can otherwise retain the existing canvas
        // surface while only the target rectangle inside it changes.
        if session.overlay_frame == overlay_frame {
            let id = session.id;
            let size = iced::Size::new(session.window_size.0, session.window_size.1);
            if let Some(session) = state.preview.as_mut() {
                session.frame = frame;
            }
            tasks.push(window::resize(id, size));
            return;
        }

        let id = session.id;
        // A monitor transition is uncommon and changes the fixed surface's
        // bounds. Update that surface only for the transition itself.
        let scale = native::dpi_scale_at_point(Point::new(overlay_frame.left, overlay_frame.top));
        let (lx, ly, lw, lh) = win::overlay::to_logical(overlay_frame, scale);
        if let Some(session) = state.preview.as_mut() {
            session.frame = frame;
            session.overlay_frame = overlay_frame;
            session.window_size = (lw, lh);
            session.scale = scale;
        }
        // This path is only for a monitor change. Keep the native fallback,
        // but never use it for ordinary quadrant/half transitions.
        if !win::overlay::set_preview_frame(overlay_frame) {
            // The HWND can briefly be unavailable while an open/close is
            // settling. Keep the logical fallback for that short window.
            tasks.push(window::move_to(id, iced::Point::new(lx, ly)));
            tasks.push(window::resize(id, iced::Size::new(lw, lh)));
        }
        return;
    }
    let scale = native::dpi_scale_at_point(Point::new(overlay_frame.left, overlay_frame.top));
    let (lx, ly, lw, lh) = win::overlay::to_logical(overlay_frame, scale);
    let (id, task) = window::open(super::windows::preview_settings(lx, ly, lw, lh));
    state.preview = Some(PreviewSession {
        id,
        frame,
        overlay_frame,
        patch_tries: 0,
        window_size: (lw, lh),
        scale,
        order_tries: 0,
    });
    tasks.push(task.map(Message::PreviewOpened));
}

/// Keep the preview surface large and stable while the target changes.
///
/// A target zone is always on one monitor, so the monitor frame is sufficient
/// to contain every half/quarter rectangle on that monitor. Using the full
/// monitor (rather than the target frame) means iced never has to resize its
/// canvas during radial hover transitions.
fn preview_overlay_frame(frame: Rect) -> Rect {
    win::monitor_service::for_rect(frame)
        .map(|monitor| monitor.monitor)
        .filter(|monitor| !monitor.is_empty())
        .unwrap_or(frame)
}

pub(super) fn close_preview(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if let Some(session) = state.preview.take() {
        tasks.push(window::close(session.id));
    }
}

pub(super) fn close_overlays(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if let Some(session) = state.radial.take() {
        tasks.push(window::close(session.id));
    }
    close_preview(state, tasks);
}

pub(super) fn commit_radial(state: &mut State, tasks: &mut Vec<Task<Message>>) {
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
            state.status = super::runtime::commit_action(state, session.target_hwnd, action, cycle);
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
                state.status =
                    super::runtime::commit_action(state, session.target_hwnd, action, cycle);
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
            let (x, y, width, height) =
                win::overlay::to_local_logical(session.frame, session.overlay_frame, session.scale);
            canvas.target = Some(Rectangle {
                x,
                y,
                width,
                height,
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::actions::WindowAction;
    use crate::core::radial_targets::RadialTargetKind;

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
    fn radial_selection_follows_angle_beyond_outer_edge() {
        let center = Point::new(100, 100);

        assert_eq!(wedge_index_at(center, 40.0, Point::new(900, 100)), Some(0));
        assert_eq!(wedge_index_at(center, 40.0, Point::new(100, -700)), Some(6));
        assert_eq!(wedge_index_at(center, 40.0, Point::new(120, 110)), None);
    }
}
