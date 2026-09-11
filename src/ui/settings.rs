use iced::Task;

use crate::core::actions::WindowAction;
use crate::core::hotkey::TriggerModifierSide;
use crate::core::monitor::MonitorMoveSizePolicy;
use crate::settings::persistence;
use crate::settings::AppSettings;
use crate::win;

use super::app::{Message, Section, State};

// --- settings persistence (edit -> disk -> shared -> services) ---
//
// Revert-on-failure keeps the edited control and the saved value in sync:
// the disk write is attempted *before* the shared mirror or services move.
// On `false` the UI reverts to the last saved snapshot (explicit reload
// strategy) so no unsaved edit lingers silently in the mirror; services keep
// running on the last good config until the next successful save.

pub(super) fn commit(state: &mut State, status: &str) {
    // Reset an in-flight press only when trigger binding or behavior
    // changed (C# resets in SetBinding/SetTriggerBehavior, never in
    // SetKeybinds): dragging a slider must not cancel a held trigger.
    // `sync::commit` computes this comparison against the last saved value
    // and reports it only on success (a failed save changes nothing).
    let last_saved = win::shared::snapshot();
    let candidate = state.settings.clone();
    let report = crate::settings::sync::commit(candidate, last_saved, persistence::save, status);
    state.settings = report.settings.clone();
    state.save_error = report.save_error;
    state.status = report.status;
    if !report.succeeded {
        win::diagnostics::report_settings(
            "LoopW couldn't save settings.json — the change was reverted. Make the file writable, then try again.",
        );
        return;
    }
    win::shared::replace(report.settings);
    if report.trigger_changed {
        win::hooks::notify_settings_changed();
    } else {
        win::hooks::refresh_config();
    }
    win::monitor_service::invalidate();
    win::stash_service::handle_settings_changed();
}

pub(super) fn try_update(state: &mut State, message: Message) -> Result<Task<Message>, Message> {
    match message {
        Message::SelectSection(section) => {
            state.section = section;
            state.confirming_reset_all = false;
            Ok(Task::none())
        }
        Message::BeginTriggerCapture => {
            state.capturing_trigger = true;
            state.capturing_keybind = None;
            win::hooks::begin_capture(win::hooks::CaptureTarget::Trigger);
            state.status = "Press a key… (Esc cancels)".to_string();
            Ok(Task::none())
        }
        Message::CancelTriggerCapture => {
            state.capturing_trigger = false;
            state.capturing_keybind = None;
            win::hooks::cancel_capture();
            state.status = "Saved".to_string();
            Ok(Task::none())
        }
        Message::BeginKeybindCapture(id) => {
            state.capturing_trigger = false;
            state.capturing_keybind = Some(id.clone());
            win::hooks::begin_capture(win::hooks::CaptureTarget::Keybind(id));
            state.status = "Press a key… (Esc cancels)".to_string();
            Ok(Task::none())
        }
        Message::SetModifierSide(side) => {
            state.settings.trigger_modifier_side = match side.as_str() {
                "Left" => TriggerModifierSide::Left,
                "Right" => TriggerModifierSide::Right,
                _ => TriggerModifierSide::Any,
            };
            commit(state, "Trigger updated");
            Ok(Task::none())
        }
        Message::ToggleKeybindCycle(id) => {
            if let Some(bind) = state.settings.keybinds.iter_mut().find(|k| k.id == id) {
                bind.cycle_enabled = !bind.cycle_enabled;
            }
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::ToggleKeybindBypass(id) => {
            if let Some(bind) = state.settings.keybinds.iter_mut().find(|k| k.id == id) {
                bind.bypass_trigger = !bind.bypass_trigger;
            }
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetLaunchAtLogin(enabled) => {
            state.settings.launch_at_login = enabled;
            if win::startup::set_launch_at_login(enabled).is_err() {
                state.settings.launch_at_login = !enabled;
                state.status = "Could not update launch setting".to_string();
                return Ok(Task::none());
            }
            commit(state, "Saved");
            // Persistence failure reverts the in-memory settings snapshot;
            // keep the registry entry aligned with that last saved value too.
            if state.settings.launch_at_login != enabled {
                let _ = win::startup::set_launch_at_login(state.settings.launch_at_login);
            }
            Ok(Task::none())
        }
        Message::NudgeDelay(delta) => {
            state.settings.trigger_delay_ms =
                (state.settings.trigger_delay_ms + delta).clamp(0, 1000);
            commit(state, "Trigger updated");
            Ok(Task::none())
        }
        Message::NudgeTimeout(delta) => {
            state.settings.trigger_timeout_ms =
                (state.settings.trigger_timeout_ms + delta).clamp(0, 10_000);
            commit(state, "Trigger updated");
            Ok(Task::none())
        }
        Message::SetDoubleClick(value) => {
            state.settings.double_click_to_trigger = value;
            commit(state, "Trigger updated");
            Ok(Task::none())
        }
        Message::SetMiddleClick(value) => {
            state.settings.middle_click_to_trigger = value;
            commit(state, "Trigger updated");
            Ok(Task::none())
        }
        Message::SetRadialEnabled(value) => {
            state.settings.radial_enabled = value;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetCursorInteraction(value) => {
            state.settings.cursor_interaction_enabled = value;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetOuterRadius(value) => {
            state.settings.radial_outer_radius = value as f64;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetInnerRadius(value) => {
            state.settings.radial_inner_radius = value as f64;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetRadialTarget(slot, choice) => {
            let target = radial_target_from_choice(&state.settings, &choice);
            match slot {
                Some(index) => {
                    if let Some(current) = state.settings.radial_slots.get_mut(index) {
                        *current = target;
                    }
                }
                None => state.settings.center_target = target,
            }
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::ToggleRadialCycle(slot) => {
            let target = match slot {
                Some(index) => state.settings.radial_slots.get_mut(index),
                None => Some(&mut state.settings.center_target),
            };
            if let Some(target) = target {
                if !matches!(
                    target.kind,
                    crate::core::radial_targets::RadialTargetKind::None
                ) {
                    target.cycle_enabled = !target.cycle_enabled;
                }
            }
            commit(state, "Saved");
            Ok(Task::none())
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
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetPreviewEnabled(value) => {
            state.settings.preview_enabled = value;
            commit(state, "Saved");
            if !value {
                let mut tasks = Vec::new();
                super::app::close_preview(state, &mut tasks);
                return Ok(Task::batch(tasks));
            }
            Ok(Task::none())
        }
        Message::SetDragSnap(value) => {
            state.settings.drag_snap_enabled = value;
            commit(state, "Saved");
            if !value {
                // End mid-drag as Disabled: hide preview, restore pre-drag
                // frame when a candidate was seen.
                let mut tasks = Vec::new();
                if let Some(finish) = win::snap_service::disable() {
                    super::runtime::apply_snap_finish(state, &mut tasks, finish);
                } else {
                    super::app::close_preview(state, &mut tasks);
                }
                return Ok(Task::batch(tasks));
            }
            Ok(Task::none())
        }
        Message::SetPreviewPadding(value) => {
            state.settings.preview_padding = value as f64;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetPreviewRadius(value) => {
            state.settings.preview_corner_radius = value as f64;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetPreviewBorder(value) => {
            state.settings.preview_border_width = value as f64;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetSnapThreshold(value) => {
            state.settings.drag_snap_threshold = value as i32;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetRestoreOnCancel(value) => {
            state.settings.restore_pre_drag_on_cancel = value;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetStashPersistence(value) => {
            state.settings.stash_persistence_enabled = value;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::NudgeStashPeek(delta) => {
            state.settings.stash_peek = (state.settings.stash_peek + delta).clamp(1, 48);
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::NudgeStashHitZone(delta) => {
            state.settings.stash_hit_zone = (state.settings.stash_hit_zone + delta).clamp(1, 96);
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::NudgeStashDelay(delta) => {
            state.settings.stash_reveal_delay_ms =
                (state.settings.stash_reveal_delay_ms + delta).clamp(0, 2000);
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetAppearanceMode(mode) => {
            state.settings.appearance_mode = mode;
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::ApplyPreset(name) => {
            if let Some(preset) = crate::ui::theme::PRESETS.iter().find(|p| p.name == name) {
                state.settings.accent_color = preset.accent.to_string();
                state.settings.radial_sector_fill = preset.sector_fill.to_string();
                state.settings.radial_sector_stroke = preset.sector_stroke.to_string();
                state.settings.radial_ring_fill = preset.ring_fill.to_string();
                state.settings.preview_border_color = preset.preview_border.to_string();
                state.color_error = None;
                commit(state, "Saved");
            }
            Ok(Task::none())
        }
        Message::EditAccent(value) => {
            apply_color_edit(state, "accent", value);
            Ok(Task::none())
        }
        Message::EditSectorFill(value) => {
            apply_color_edit(state, "sector_fill", value);
            Ok(Task::none())
        }
        Message::EditSectorStroke(value) => {
            apply_color_edit(state, "sector_stroke", value);
            Ok(Task::none())
        }
        Message::EditRingFill(value) => {
            apply_color_edit(state, "ring_fill", value);
            Ok(Task::none())
        }
        Message::EditPreviewBorderColor(value) => {
            apply_color_edit(state, "preview_border", value);
            Ok(Task::none())
        }
        Message::AddKeybind => {
            state
                .settings
                .keybinds
                .push(crate::settings::Keybind::default());
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::DeleteKeybind(id) => {
            state.settings.keybinds.retain(|k| k.id != id);
            commit(state, "Saved");
            Ok(Task::none())
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
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::SetMonitorPolicy(policy) => {
            state.settings.monitor_move_policy = if policy == "PreserveLogicalSize" {
                MonitorMoveSizePolicy::PreserveLogicalSize
            } else {
                MonitorMoveSizePolicy::PreservePixels
            };
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::NudgeGlobalPadding(delta) => {
            state.settings.global_padding = (state.settings.global_padding + delta).clamp(0, 128);
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::NudgePaddingLeft(delta) => {
            state.settings.padding_left = (state.settings.padding_left + delta).clamp(0, 128);
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::NudgePaddingTop(delta) => {
            state.settings.padding_top = (state.settings.padding_top + delta).clamp(0, 128);
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::NudgePaddingRight(delta) => {
            state.settings.padding_right = (state.settings.padding_right + delta).clamp(0, 128);
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::NudgePaddingBottom(delta) => {
            state.settings.padding_bottom = (state.settings.padding_bottom + delta).clamp(0, 128);
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::EditExcludedExecutables(value) => {
            state.settings.excluded_executables = value
                .split([',', '\n'])
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::EditExcludedProcesses(value) => {
            state.settings.excluded_processes = value
                .split([',', '\n'])
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            commit(state, "Saved");
            Ok(Task::none())
        }
        Message::ResetSection(section) => {
            reset_section(state, section);
            commit(state, "Reset complete");
            Ok(Task::none())
        }
        Message::ConfirmResetAll => {
            state.confirming_reset_all = true;
            Ok(Task::none())
        }
        Message::CancelResetAll => {
            state.confirming_reset_all = false;
            Ok(Task::none())
        }
        Message::ResetAll => {
            state.settings.reset_all();
            state.color_error = None;
            state.confirming_reset_all = false;
            commit(state, "Reset complete");
            Ok(Task::none())
        }
        Message::ClearDiagnostics => {
            win::diagnostics::clear();
            state.status = "Diagnostics cleared".to_string();
            Ok(Task::none())
        }
        message => Err(message),
    }
}

/// Reject a captured keybind that collides with the trigger or duplicates
/// another row (C# HasKeybindConflict): the colliding bind would be
/// permanently inert (trigger-Vk exclusion) or shadowed (first-wins).
pub(super) fn keybind_conflict(
    settings: &AppSettings,
    skip_id: &str,
    modifiers: u32,
    vk: u32,
) -> Option<String> {
    use crate::core::hotkey::hotkey_name;
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
    commit(state, "Saved");
}

pub(super) fn radial_target_from_choice(
    settings: &AppSettings,
    choice: &str,
) -> crate::core::radial_targets::RadialTargetSettings {
    use crate::core::radial_targets::{RadialTargetKind, RadialTargetSettings};

    if choice == "No action" {
        return RadialTargetSettings::none();
    }
    if let Some(name) = choice.strip_prefix("Action: ") {
        if let Some(action) = WindowAction::ALL
            .iter()
            .copied()
            .find(|action| action.display_name() == name)
        {
            return RadialTargetSettings::action(action);
        }
    }
    if let Some(rest) = choice.strip_prefix("Keybind: ") {
        let id = rest.split(" (").next().unwrap_or(rest);
        if let Some(bind) = settings.keybinds.iter().find(|bind| bind.id == id) {
            return RadialTargetSettings {
                kind: RadialTargetKind::Keybind,
                action: bind.action,
                keybind_id: bind.id.clone(),
                cycle_enabled: bind.cycle_enabled,
            };
        }
    }
    RadialTargetSettings::none()
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
            state.settings.stash_persistence_enabled = defaults.stash_persistence_enabled;
            state.settings.stash_peek = defaults.stash_peek;
            state.settings.stash_hit_zone = defaults.stash_hit_zone;
            state.settings.stash_reveal_delay_ms = defaults.stash_reveal_delay_ms;
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
        }
    }
}
