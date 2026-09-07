//! Appearance section: presets first, advanced color editor inline.

use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};

use crate::ui::app::{Message, Section, State};
use crate::ui::theme::{matching_preset, PRESETS};

pub fn view(state: &State) -> Element<'_, Message> {
    let settings = &state.settings;
    let current = matching_preset(
        &settings.accent_color,
        &settings.radial_sector_fill,
        &settings.radial_sector_stroke,
        &settings.radial_ring_fill,
        &settings.preview_border_color,
    )
    .unwrap_or("Custom");

    let names: Vec<String> = PRESETS.iter().map(|p| p.name.to_string()).collect();
    let modes = vec![
        "Dark".to_string(),
        "FollowWindows".to_string(),
        "Light".to_string(),
    ];

    let content = column![
        text("Appearance").size(20),
        text("Dark is the product standard. Presets apply instantly.").size(13),
        row![
            text("Theme").size(14),
            pick_list(
                modes,
                Some(settings.appearance_mode.clone()),
                Message::SetAppearanceMode,
            ),
        ]
        .spacing(12),
        row![
            text(format!("Preset: {current}")).size(14),
            pick_list(names, Some(current.to_string()), Message::ApplyPreset),
        ]
        .spacing(12),
        color_row("Accent", &settings.accent_color, Message::EditAccent),
        color_row(
            "Sector fill",
            &settings.radial_sector_fill,
            Message::EditSectorFill
        ),
        color_row(
            "Sector stroke",
            &settings.radial_sector_stroke,
            Message::EditSectorStroke
        ),
        color_row(
            "Ring fill",
            &settings.radial_ring_fill,
            Message::EditRingFill
        ),
        color_row(
            "Preview border",
            &settings.preview_border_color,
            Message::EditPreviewBorderColor
        ),
        if let Some(error) = &state.color_error {
            text(error).size(12)
        } else {
            text("Colors accept #RRGGBB or #AARRGGBB.").size(12)
        },
        row![button("Reset section").on_press(Message::ResetSection(Section::Appearance)),],
        text(&state.status).size(12),
    ]
    .spacing(10)
    .padding(16);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn color_row<'a>(
    label: &'static str,
    value: &str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    use crate::settings::color::to_iced;
    let swatch = match to_iced(value) {
        Some(c) => format!(
            "■ {:02X}{:02X}{:02X}",
            (c.r * 255.0) as u8,
            (c.g * 255.0) as u8,
            (c.b * 255.0) as u8
        ),
        None => "■ invalid".to_string(),
    };
    iced::widget::row![
        text(format!("{label}  {swatch}")).size(13),
        text_input("e.g. #3D9BFF", value).on_input(on_input),
    ]
    .spacing(10)
    .into()
}
