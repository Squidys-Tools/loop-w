//! Main iced settings application: left nav + content pane.
//!
//! General is the default section so trigger/launch stay prominent. Every
//! edit auto-saves through the atomic persistence layer and reports one of
//! the small status states (Saved / Could not save / Trigger updated /
//! Reset complete) without flickering on every slider tick.

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length, Theme};

use crate::core::hotkey::TriggerModifierSide;
use crate::core::monitor::MonitorMoveSizePolicy;
use crate::core::radial_targets::RadialTargetKind;
use crate::settings::AppSettings;
use crate::settings::persistence;

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
    SetMonitorPolicy(String),
    NudgeGlobalPadding(i32),
    EditExcludedProcesses(String),
    ResetSection(Section),
    ConfirmResetAll,
    CancelResetAll,
    ResetAll,
}

/// Mutable UI state.
pub struct State {
    pub settings: AppSettings,
    pub section: Section,
    pub status: String,
    pub color_error: Option<String>,
    pub capturing_trigger: bool,
    pub confirming_reset_all: bool,
}

impl State {
    pub fn boot() -> Self {
        Self {
            settings: persistence::load(),
            section: Section::General,
            status: "Saved".to_string(),
            color_error: None,
            capturing_trigger: false,
            confirming_reset_all: false,
        }
    }

    fn persist(&mut self, ok_status: &str) {
        if persistence::save(&mut self.settings) {
            self.status = ok_status.to_string();
        } else {
            self.status = "Could not save".to_string();
        }
    }
}

pub fn run() -> iced::Result {
    iced::application(State::boot, update, view)
        .title(|state: &State| format!("LoopW Settings — {}", state.section.label()))
        .window_size(iced::Size::new(1166.0, 779.0))
        .centered()
        .theme(|state: &State| {
            if state.settings.is_light() {
                Theme::Light
            } else {
                Theme::Dark
            }
        })
        .run()
}

fn update(state: &mut State, message: Message) {
    match message {
        Message::Noop => {}
        Message::SelectSection(section) => {
            state.section = section;
            state.confirming_reset_all = false;
        }
        Message::BeginTriggerCapture => {
            state.capturing_trigger = true;
            state.status = "Press a key… (Esc cancels)".to_string();
        }
        Message::CancelTriggerCapture => {
            state.capturing_trigger = false;
            state.status = "Saved".to_string();
        }
        Message::SetLaunchAtLogin(enabled) => {
            state.settings.launch_at_login = enabled;
            if crate::win::startup::set_launch_at_login(enabled).is_err() {
                state.settings.launch_at_login = !enabled;
                state.status = "Could not update launch setting".to_string();
                return;
            }
            state.persist("Saved");
        }
        Message::NudgeDelay(delta) => {
            state.settings.trigger_delay_ms =
                (state.settings.trigger_delay_ms + delta).clamp(0, 1000);
            state.persist("Trigger updated");
        }
        Message::NudgeTimeout(delta) => {
            state.settings.trigger_timeout_ms =
                (state.settings.trigger_timeout_ms + delta).clamp(0, 10_000);
            state.persist("Trigger updated");
        }
        Message::SetDoubleClick(value) => {
            state.settings.double_click_to_trigger = value;
            state.persist("Trigger updated");
        }
        Message::SetMiddleClick(value) => {
            state.settings.middle_click_to_trigger = value;
            state.persist("Trigger updated");
        }
        Message::SetRadialEnabled(value) => {
            state.settings.radial_enabled = value;
            state.persist("Saved");
        }
        Message::SetCursorInteraction(value) => {
            state.settings.cursor_interaction_enabled = value;
            state.persist("Saved");
        }
        Message::SetOuterRadius(value) => {
            state.settings.radial_outer_radius = value as f64;
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::SetInnerRadius(value) => {
            state.settings.radial_inner_radius = value as f64;
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::ClearWedge(index) => {
            if let Some(slot) = state.settings.radial_slots.get_mut(index) {
                *slot = crate::core::radial_targets::RadialTargetSettings {
                    kind: RadialTargetKind::None,
                    action: crate::core::actions::WindowAction::RightHalf,
                    keybind_id: String::new(),
                    cycle_enabled: false,
                };
            }
            state.persist("Saved");
        }
        Message::SetPreviewEnabled(value) => {
            state.settings.preview_enabled = value;
            state.persist("Saved");
        }
        Message::SetDragSnap(value) => {
            state.settings.drag_snap_enabled = value;
            state.persist("Saved");
        }
        Message::SetPreviewPadding(value) => {
            state.settings.preview_padding = value as f64;
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::SetPreviewRadius(value) => {
            state.settings.preview_corner_radius = value as f64;
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::SetPreviewBorder(value) => {
            state.settings.preview_border_width = value as f64;
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::SetSnapThreshold(value) => {
            state.settings.drag_snap_threshold = value as i32;
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::SetRestoreOnCancel(value) => {
            state.settings.restore_pre_drag_on_cancel = value;
            state.persist("Saved");
        }
        Message::SetAppearanceMode(mode) => {
            state.settings.appearance_mode = mode;
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::ApplyPreset(name) => {
            if let Some(preset) = crate::ui::theme::PRESETS.iter().find(|p| p.name == name) {
                state.settings.accent_color = preset.accent.to_string();
                state.settings.radial_sector_fill = preset.sector_fill.to_string();
                state.settings.radial_sector_stroke = preset.sector_stroke.to_string();
                state.settings.radial_ring_fill = preset.ring_fill.to_string();
                state.settings.preview_border_color = preset.preview_border.to_string();
                state.color_error = None;
                state.persist("Saved");
            }
        }
        Message::EditAccent(value) => apply_color_edit(state, "accent", value),
        Message::EditSectorFill(value) => apply_color_edit(state, "sector_fill", value),
        Message::EditSectorStroke(value) => apply_color_edit(state, "sector_stroke", value),
        Message::EditRingFill(value) => apply_color_edit(state, "ring_fill", value),
        Message::EditPreviewBorderColor(value) => {
            apply_color_edit(state, "preview_border", value);
        }
        Message::AddKeybind => {
            state.settings.keybinds.push(crate::settings::Keybind::default());
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::DeleteKeybind(id) => {
            state.settings.keybinds.retain(|k| k.id != id);
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::SetMonitorPolicy(policy) => {
            state.settings.monitor_move_policy = if policy == "PreserveLogicalSize" {
                MonitorMoveSizePolicy::PreserveLogicalSize
            } else {
                MonitorMoveSizePolicy::PreservePixels
            };
            state.persist("Saved");
        }
        Message::NudgeGlobalPadding(delta) => {
            state.settings.global_padding =
                (state.settings.global_padding + delta).clamp(0, 128);
            state.persist("Saved");
        }
        Message::EditExcludedProcesses(value) => {
            state.settings.excluded_processes = value
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            state.settings.normalize();
            state.persist("Saved");
        }
        Message::ResetSection(section) => {
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
                    state.settings.cursor_interaction_enabled =
                        defaults.cursor_interaction_enabled;
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
                    state.settings.restore_pre_drag_on_cancel =
                        defaults.restore_pre_drag_on_cancel;
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
            state.settings.normalize();
            state.persist("Reset complete");
        }
        Message::ConfirmResetAll => {
            state.confirming_reset_all = true;
        }
        Message::CancelResetAll => {
            state.confirming_reset_all = false;
        }
        Message::ResetAll => {
            state.settings.reset_all();
            state.settings.normalize();
            state.confirming_reset_all = false;
            state.color_error = None;
            state.persist("Reset complete");
        }
    }
}

// Color edits need the field identity; handle them with dedicated helpers so
// validation errors stay inline and invalid values never corrupt the file.
#[allow(dead_code)]
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
    state.settings.normalize();
    state.persist("Saved");
}

fn view(state: &State) -> Element<'_, Message> {
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
        container(scrollable(content)).width(Length::Fill).height(Length::Fill),
    ];

    container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
