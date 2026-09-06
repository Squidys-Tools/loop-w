//! General section: trigger + launch behavior first.

use iced::widget::{button, column, container, row, text, toggler};
use iced::{Element, Length};

use crate::core::hotkey::hotkey_name;
use crate::ui::app::{Message, Section, State};

pub fn view(state: &State) -> Element<'_, Message> {
    let settings = &state.settings;
    let trigger = hotkey_name(
        settings.trigger_modifiers,
        settings.trigger_vk,
        settings.trigger_modifier_side,
    );
    let capture_label = if state.capturing_trigger {
        "Press a key… (Esc cancels)"
    } else {
        "Rebind trigger"
    };

    let content = column![
        text("Trigger and launch").size(20),
        text("Hold the trigger, flick toward a direction, release to place the window.").size(13),
        row![
            text(format!("Trigger: {trigger}")).size(15),
            button(capture_label).on_press(Message::BeginTriggerCapture),
        ]
        .spacing(12),
        if state.capturing_trigger {
            text("Press a key or combination. Esc cancels.").size(12)
        } else {
            text("").size(1)
        },
        row![
            text("Launch at login").size(15),
            toggler(settings.launch_at_login).on_toggle(Message::SetLaunchAtLogin),
        ]
        .spacing(12),
        row![
            text(format!("Activation delay: {} ms", settings.trigger_delay_ms)).size(14),
            button("-").on_press(Message::NudgeDelay(-50)),
            button("+").on_press(Message::NudgeDelay(50)),
        ]
        .spacing(8),
        row![
            text(format!(
                "Release timeout: {}",
                if settings.trigger_timeout_ms == 0 {
                    "Off".to_string()
                } else {
                    format!("{} ms", settings.trigger_timeout_ms)
                }
            ))
            .size(14),
            button("-").on_press(Message::NudgeTimeout(-500)),
            button("+").on_press(Message::NudgeTimeout(500)),
        ]
        .spacing(8),
        row![
            text("Double-click to trigger").size(14),
            toggler(settings.double_click_to_trigger)
                .on_toggle(Message::SetDoubleClick),
        ]
        .spacing(12),
        row![
            text("Middle-mouse to trigger").size(14),
            toggler(settings.middle_click_to_trigger)
                .on_toggle(Message::SetMiddleClick),
        ]
        .spacing(12),
        row![
            button("Reset section").on_press(Message::ResetSection(Section::General)),
        ],
        text(&state.status).size(12),
    ]
    .spacing(12)
    .padding(16);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
