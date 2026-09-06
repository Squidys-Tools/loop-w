//! Advanced section: keybinds, monitor policy, padding, exclusions, reset-all.

use iced::widget::{button, column, container, pick_list, row, text, text_input, toggler};
use iced::{Element, Length};

use crate::core::actions::WindowAction;
use crate::core::hotkey::hotkey_name;
use crate::core::hotkey::TriggerModifierSide;
use crate::ui::app::{Message, Section, State};

pub fn view(state: &State) -> Element<'_, Message> {
    let settings = &state.settings;

    let mut binds = column![text("Keybinds").size(15)].spacing(6);
    if settings.keybinds.is_empty() {
        binds = binds.push(text("No keybinds yet. Keybinds run without opening the radial menu.").size(13));
    }
    for bind in &settings.keybinds {
        let label = format!(
            "{} → {}",
            hotkey_name(bind.modifiers, bind.vk, TriggerModifierSide::Any),
            bind.action.display_name()
        );
        binds = binds.push(
            row![
                text(label).size(13),
                button("Delete").on_press(Message::DeleteKeybind(bind.id.clone())),
            ]
            .spacing(10),
        );
    }
    binds = binds.push(
        row![
            button("Add keybind").on_press(Message::AddKeybind),
        ]
        .spacing(10),
    );

    let content = column![
        text("Advanced").size(20),
        text("Power-user controls. Routine settings live in the other sections.").size(13),
        binds,
        row![
            text("Monitor move sizing").size(14),
            pick_list(
                vec![
                    "PreservePixels".to_string(),
                    "PreserveLogicalSize".to_string(),
                ],
                Some(
                    match settings.monitor_move_policy {
                        crate::core::monitor::MonitorMoveSizePolicy::PreservePixels =>
                            "PreservePixels".to_string(),
                        crate::core::monitor::MonitorMoveSizePolicy::PreserveLogicalSize =>
                            "PreserveLogicalSize".to_string(),
                    }
                ),
                Message::SetMonitorPolicy,
            ),
        ]
        .spacing(12),
        row![
            text(format!("Global padding: {}", settings.global_padding)).size(14),
            button("-").on_press(Message::NudgeGlobalPadding(-1)),
            button("+").on_press(Message::NudgeGlobalPadding(1)),
        ]
        .spacing(8),
        row![
            text("Excluded processes (one per line, e.g. game.exe)").size(13),
        ],
        text_input(
            "excluded.exe",
            &settings.excluded_processes.join("\n"),
        )
        .on_input(Message::EditExcludedProcesses),
        row![
            text("Stash peek / hit zone / delay").size(13),
            button("Defaults").on_press(Message::ResetSection(Section::Advanced)),
        ],
        row![
            text(format!(
                "Bypass note: {} keybind(s) bypass the trigger.",
                settings.keybinds.iter().filter(|k| k.bypass_trigger).count()
            ))
            .size(12),
        ],
        row![
            button("Reset section").on_press(Message::ResetSection(Section::Advanced)),
            button("Reset ALL settings…").on_press(Message::ConfirmResetAll),
        ]
        .spacing(12),
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
        text(format!("Window actions available: {}", WindowAction::ALL.len())).size(11),
        row![
            text("Bypass toggles and per-edge padding editors live here next.").size(11),
            toggler(false).on_toggle(|_| Message::Noop),
        ],
    ]
    .spacing(10)
    .padding(16);

    container(content).width(Length::Fill).height(Length::Fill).into()
}
