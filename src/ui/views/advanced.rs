//! Advanced section: keybinds, monitor policy, padding, exclusions, reset-all.

use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use iced::widget::{
    button, column, container, keyed_column, pick_list, row, text, text_input, toggler,
};
use iced::{Element, Length};

use crate::core::actions::WindowAction;
use crate::core::hotkey::hotkey_name;
use crate::core::hotkey::TriggerModifierSide;
use crate::ui::app::{Message, Section, State};

const MONITOR_MOVE_POLICIES: [&str; 2] = ["PreservePixels", "PreserveLogicalSize"];
const KEYBIND_HEADER_KEY: u64 = 0;
const KEYBIND_EMPTY_KEY: u64 = 1;
const KEYBIND_ADD_KEY: u64 = 2;

pub fn view(state: &State) -> Element<'_, Message> {
    let settings = &state.settings;

    let mut binds =
        keyed_column([(KEYBIND_HEADER_KEY, text("Keybinds").size(15).into())]).spacing(6);
    if settings.keybinds.is_empty() {
        binds = binds.push(
            KEYBIND_EMPTY_KEY,
            text("No keybinds yet. Keybinds run without opening the radial menu.").size(13),
        );
    }
    let actions = action_choices();
    for bind in &settings.keybinds {
        let id = bind.id.as_str();
        let capturing = state.capturing_keybind.as_deref() == Some(bind.id.as_str());
        let key_label = if capturing {
            "Press a key… (Esc cancels)".to_string()
        } else {
            hotkey_name(bind.modifiers, bind.vk, TriggerModifierSide::Any)
        };
        binds = binds.push(
            keybind_row_key(&bind.id),
            column![
                row![
                    button(text(key_label).size(13))
                        .on_press(Message::BeginKeybindCapture(bind.id.clone())),
                    pick_list(actions, Some(bind.action.display_name()), {
                        move |name: &str| {
                            Message::SetKeybindAction(id.to_string(), name.to_string())
                        }
                    },),
                    button("Delete").on_press(Message::DeleteKeybind(bind.id.clone())),
                ]
                .spacing(10),
                row![
                    text("Cycle").size(12),
                    toggler(bind.cycle_enabled)
                        .on_toggle(move |_| Message::ToggleKeybindCycle(id.to_string())),
                    text("Bypass trigger").size(12),
                    toggler(bind.bypass_trigger)
                        .on_toggle(move |_| Message::ToggleKeybindBypass(id.to_string())),
                ]
                .spacing(10),
            ]
            .spacing(4),
        );
    }
    binds = binds.push(
        KEYBIND_ADD_KEY,
        row![button("Add keybind").on_press(Message::AddKeybind)].spacing(10),
    );

    let content = column![
        text("Advanced").size(20),
        text("Power-user controls. Routine settings live in the other sections.").size(13),
        binds,
        row![
            text("Monitor move sizing").size(14),
            pick_list(
                &MONITOR_MOVE_POLICIES[..],
                Some(match settings.monitor_move_policy {
                    crate::core::monitor::MonitorMoveSizePolicy::PreservePixels => {
                        "PreservePixels"
                    }
                    crate::core::monitor::MonitorMoveSizePolicy::PreserveLogicalSize => {
                        "PreserveLogicalSize"
                    }
                }),
                |policy: &str| Message::SetMonitorPolicy(policy.to_string()),
            ),
        ]
        .spacing(12),
        row![
            text(format!("Global padding: {}", settings.global_padding)).size(14),
            button("-").on_press(Message::NudgeGlobalPadding(-1)),
            button("+").on_press(Message::NudgeGlobalPadding(1)),
        ]
        .spacing(8),
        text("Per-edge screen padding").size(14),
        padding_row("Left", settings.padding_left, Message::NudgePaddingLeft),
        padding_row("Top", settings.padding_top, Message::NudgePaddingTop),
        padding_row("Right", settings.padding_right, Message::NudgePaddingRight),
        padding_row(
            "Bottom",
            settings.padding_bottom,
            Message::NudgePaddingBottom
        ),
        row![text("Excluded executables (comma-separated paths)").size(13),],
        text_input(
            "C:\\Games\\example.exe, C:\\Tools\\demo.exe",
            &settings.excluded_executables.join(", "),
        )
        .on_input(Message::EditExcludedExecutables),
        row![text("Excluded processes (comma-separated names)").size(13),],
        text_input(
            "game.exe, editor.exe",
            &settings.excluded_processes.join(", "),
        )
        .on_input(Message::EditExcludedProcesses),
        row![text("Stash settings are on the Preview section.").size(13),],
        row![text(format!(
            "Bypass note: {} keybind(s) bypass the trigger.",
            settings
                .keybinds
                .iter()
                .filter(|k| k.bypass_trigger)
                .count()
        ))
        .size(12),],
        row![
            button("Reset section").on_press(Message::ResetSection(Section::Advanced)),
            button("Reset ALL settings…").on_press(Message::ConfirmResetAll),
        ]
        .spacing(12),
        super::diagnostics::section(),
        if state.confirming_reset_all {
            row![
                text("Reset everything to defaults?").size(13),
                button("Confirm reset").on_press(Message::ResetAll),
                button("Cancel").on_press(Message::CancelResetAll),
            ]
            .spacing(10)
        } else {
            row![].spacing(0)
        },
        text(&state.status).size(12),
        text(format!(
            "Window actions available: {}",
            WindowAction::ALL.len()
        ))
        .size(11),
    ]
    .spacing(10)
    .padding(16);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn action_choices() -> &'static [&'static str] {
    static CHOICES: OnceLock<Box<[&'static str]>> = OnceLock::new();

    CHOICES
        .get_or_init(|| {
            WindowAction::ALL
                .iter()
                .map(|action| action.display_name())
                .collect()
        })
        .as_ref()
}

fn keybind_row_key(id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish() | (1 << 63)
}

fn padding_row(
    label: &'static str,
    value: i32,
    on_change: fn(i32) -> Message,
) -> Element<'static, Message> {
    row![
        text(format!("{label}: {value}")).size(13),
        button("-").on_press(on_change(-1)),
        button("+").on_press(on_change(1)),
    ]
    .spacing(8)
    .into()
}
