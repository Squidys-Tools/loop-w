//! Radial section: text-free surface first, geometry + assignments below.

use iced::widget::{button, column, container, row, slider, text, toggler};
use iced::{Element, Length};

use crate::core::radial::GEOMETRY;
use crate::ui::app::{Message, Section, State};
use crate::ui::widgets::radial_canvas::{self, RadialCanvas};

pub fn view(state: &State) -> Element<'_, Message> {
    let settings = &state.settings;
    let canvas = RadialCanvas::from_settings(settings);

    let mut assignments = column![text("Wedge assignments").size(15)].spacing(6);
    for (index, slot) in GEOMETRY.iter().enumerate() {
        let target = settings.radial_slots.get(index);
        let name = target
            .map(|t| match t.kind {
                crate::core::radial_targets::RadialTargetKind::None => "No action".to_string(),
                crate::core::radial_targets::RadialTargetKind::Action => {
                    t.action.display_name().to_string()
                }
                crate::core::radial_targets::RadialTargetKind::Keybind => {
                    format!("Keybind {}", t.keybind_id.chars().take(6).collect::<String>())
                }
            })
            .unwrap_or_else(|| "No action".to_string());
        assignments = assignments.push(
            row![
                text(format!("{} — {name}", slot.label)).size(13),
                button("Clear").on_press(Message::ClearWedge(index)),
            ]
            .spacing(10),
        );
    }

    let content = column![
        text("Radial menu").size(20),
        text("The overlay itself never shows text — labels here are settings-only.").size(13),
        radial_canvas::view(canvas, 260.0),
        row![
            text("Enabled").size(14),
            toggler(settings.radial_enabled).on_toggle(Message::SetRadialEnabled),
            text("Cursor follows direction").size(14),
            toggler(settings.cursor_interaction_enabled)
                .on_toggle(Message::SetCursorInteraction),
        ]
        .spacing(12),
        text(format!("Outer radius: {:.0} px", settings.radial_outer_radius)).size(14),
        slider(
            64.0..=140.0,
            settings.radial_outer_radius as f32,
            Message::SetOuterRadius
        ),
        text(format!("Inner radius: {:.0} px", settings.radial_inner_radius)).size(14),
        slider(
            24.0..=132.0,
            settings.radial_inner_radius as f32,
            Message::SetInnerRadius
        ),
        assignments,
        row![
            button("Reset section").on_press(Message::ResetSection(Section::Radial)),
        ],
        text(&state.status).size(12),
    ]
    .spacing(12)
    .padding(16);

    container(content).width(Length::Fill).height(Length::Fill).into()
}
