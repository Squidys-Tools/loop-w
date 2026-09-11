use std::time::Duration;

use iced::{keyboard, window, Subscription, Task};

use crate::core::actions::WindowAction;
use crate::core::hotkey::hotkey_name;
use crate::win::events::{drain, RuntimeEvent};
use crate::win::{self, native};

use super::app::{self, Message, State};
use super::settings;
use super::windows;

pub(super) fn subscription(state: &State) -> Subscription<Message> {
    let settings_only = state.main_id.is_some()
        && state.radial.is_none()
        && state.preview.is_none()
        && !win::snap_service::is_active();

    let mut subscriptions = vec![window::close_requests().map(Message::WindowClosed)];
    subscriptions.push(iced::event::listen_with(|event, status, _window| {
        if status != iced::event::Status::Ignored {
            return None;
        }
        match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                Some(Message::OverlayKey(key))
            }
            _ => None,
        }
    }));

    if settings_only {
        // A settings-only window does not need animation frames. Keeping the
        // 60 Hz frame subscription here makes every pump tick rebuild the
        // entire settings tree, which makes Scrollable feel laggy. Keep the
        // resident tray/event pump alive at a low rate instead.
        subscriptions
            .push(iced::time::every(Duration::from_millis(250)).map(|_| Message::FrameTick));
    } else {
        // Overlay hover and drag snapping need frame-rate cursor tracking. A
        // timer is sufficient for that polling; subscribing to window frames
        // as well duplicates ticks because each update can request a redraw.
        // The same timer also keeps the tray-only runtime alive, where iced
        // has no window redraw events to drive it.
        subscriptions
            .push(iced::time::every(Duration::from_millis(16)).map(|_| Message::FrameTick));
    }

    Subscription::batch(subscriptions)
}

/// Once-per-frame pump: events, tray, stash, snap, overlay hover, patching.
pub(super) fn frame_tick(state: &mut State) -> Task<Message> {
    let mut tasks: Vec<Task<Message>> = Vec::new();
    if state.main_id.is_none() {
        if let Some(hwnd) = native::find_window_by_class(native::WINIT_EVENT_TARGET_CLASS) {
            native::suppress_taskbar_window(hwnd);
        }
    }
    if let Some(tray) = state.tray.as_ref() {
        win::tray::poll(tray);
    }
    // Stash records change under us (persist/prune paths): mirror them only
    // while settings is open, and only when the value actually changed. The
    // shared snapshot clones the full settings object, so avoid doing that on
    // every overlay tick; the settings-only pump is deliberately slower.
    if state.main_id.is_some() {
        let stash_records = win::shared::stash_records();
        if state.settings.stash_records != stash_records {
            state.settings.stash_records = stash_records;
        }
    }
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
                    app::ensure_preview(state, &mut tasks, frame);
                }
            }
            win::snap_service::SnapTrack::Hide => app::close_preview(state, &mut tasks),
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
        app::update_radial_hover(state, &mut tasks, cursor);
    }
    // Overlay style patching retries (windows can appear several ticks after
    // open, especially when the preview follows a newly opened radial).
    // Keep this bounded at roughly one second while avoiding a preview that
    // briefly becomes a taskbar/Alt+Tab window during async creation.
    const OVERLAY_PATCH_ATTEMPTS: u8 = 60;
    if let Some(session) = state.radial.as_mut() {
        if session.patch_tries < OVERLAY_PATCH_ATTEMPTS
            && win::overlay::patch_tool_window(app::radial_expected(session))
        {
            session.patch_tries = OVERLAY_PATCH_ATTEMPTS;
        } else if session.patch_tries < OVERLAY_PATCH_ATTEMPTS {
            session.patch_tries += 1;
        }
    }
    if let Some(session) = state.preview.as_mut() {
        if session.patch_tries < OVERLAY_PATCH_ATTEMPTS
            && win::overlay::patch_click_through(session.overlay_frame)
        {
            session.patch_tries = OVERLAY_PATCH_ATTEMPTS;
        } else if session.patch_tries < OVERLAY_PATCH_ATTEMPTS {
            session.patch_tries += 1;
        }
    }
    // The preview opens and moves after the radial, so it settles above it
    // in the topmost band and hides the menu where they overlap. Pin the
    // preview back below the radial (no activation, no focus steal), but only
    // retry while the asynchronous windows are settling. Re-running the
    // EnumWindows lookup every tick made the overlay path needlessly expensive.
    if let (Some(radial), Some(preview)) = (state.radial.as_ref(), state.preview.as_mut()) {
        if preview.order_tries < 12 {
            let ordered = win::overlay::order_preview_below_radial(
                preview.overlay_frame,
                app::radial_expected(radial),
            );
            if ordered {
                preview.order_tries = 12;
            } else {
                preview.order_tries += 1;
            }
        }
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
                    app::open_radial(state, tasks, foreground);
                }
            }
        }
        RuntimeEvent::TriggerReleased => {
            state.pending_target = None;
            if state.radial.is_some() {
                app::commit_radial(state, tasks);
            }
        }
        RuntimeEvent::TriggerTimedOut => {
            state.pending_target = None;
            app::close_overlays(state, tasks);
            state.status = "Trigger timed out — no action committed".to_string();
        }
        RuntimeEvent::TriggerCancelled => {
            state.pending_target = None;
            app::close_overlays(state, tasks);
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
            app::close_overlays(state, tasks);
            state.status = commit_action(state, target, action, cycle_enabled);
        }
        RuntimeEvent::RevealStashed => {
            app::close_overlays(state, tasks);
            state.status = win::stash_service::reveal_next().unwrap_or_else(|error| error);
        }
        RuntimeEvent::CaptureUpdate {
            modifiers,
            vk,
            keybind,
        } => match keybind {
            Some(id) => {
                if let Some(reason) =
                    settings::keybind_conflict(&state.settings, &id, modifiers, vk)
                {
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
                settings::commit(state, "Keybind updated");
            }
            None => {
                state.settings.trigger_modifiers = modifiers;
                state.settings.trigger_vk = vk;
                state.capturing_trigger = false;
                state.capturing_keybind = None;
                settings::commit(state, "Trigger updated");
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
            windows::show_main(state, tasks);
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
            app::close_preview(state, tasks);
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
                app::close_preview(state, tasks);
            }
        }
        RuntimeEvent::Diagnostic { message, .. } => {
            // The entry is already in the diagnostics log (Advanced
            // section); mirror the friendly message in the status line so
            // background failures are visible without opening it.
            state.status = message;
        }
    }
}

pub(super) fn commit_action(
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

pub(super) fn apply_snap_finish(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    finish: win::snap_service::SnapFinish,
) {
    app::close_preview(state, tasks);
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

// --- command execution (IPC + startup command) ---

pub(super) fn run_startup_command(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if let Some(command) = state.startup_command.take() {
        let reply = execute_command(state, tasks, &command);
        native::print_cli_line(&reply);
    }
}

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
                windows::show_main(state, tasks);
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
