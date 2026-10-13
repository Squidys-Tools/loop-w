//! Appearance section: presets first, advanced color editor inline.

use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Element, Length};

use crate::core::motion::MotionPreference;
use crate::ui::app::{Message, Section, State};
use crate::ui::theme::{matching_preset, PRESET_NAMES};

const APPEARANCE_MODES: [&str; 3] = ["Dark", "FollowWindows", "Light"];
const MOTION_MODES: [&str; 3] = ["Follow Windows", "Reduced motion", "Full motion"];

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

    let content = column![
        text("Appearance").size(20),
        text("Dark is the product standard. Presets apply instantly.").size(13),
        row![
            text("Theme").size(14),
            pick_list(
                &APPEARANCE_MODES[..],
                Some(settings.appearance_mode.as_str()),
                |mode: &str| Message::SetAppearanceMode(mode.to_string()),
            ),
        ]
        .spacing(12),
        row![
            text("Motion").size(14),
            pick_list(
                &MOTION_MODES[..],
                Some(settings.reduce_motion.label()),
                |mode: &str| Message::SetMotionPreference(mode.to_string()),
            ),
        ]
        .spacing(12),
        text(motion_note(state)).size(12),
        row![
            text(format!("Preset: {current}")).size(14),
            pick_list(&PRESET_NAMES[..], Some(current), |preset: &str| {
                Message::ApplyPreset(preset.to_string())
            }),
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

/// Resolved motion state in one sentence-case line.
///
/// LoopW's surfaces are instant today, so the line reports what the
/// preference resolves to rather than implying a live effect. Nonessential
/// transitions consult `reduced_motion` as they are added.
fn motion_note(state: &State) -> String {
    match state.settings.reduce_motion {
        MotionPreference::FollowWindows => {
            if crate::ui::runtime::reduced_motion(state) {
                "LoopW is set to reduce motion while following Windows.".to_string()
            } else {
                "LoopW is set to allow motion while following Windows.".to_string()
            }
        }
        MotionPreference::Reduced => {
            "Motion stays reduced even when Windows allows it.".to_string()
        }
        MotionPreference::FullMotion => {
            "Motion stays full even when Windows asks to reduce it.".to_string()
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::motion::MOTION_PREFERENCES;

    #[test]
    fn motion_modes_match_the_persisted_labels() {
        // The pick list shows these strings; `MotionPreference::from_label`
        // parses them back. A rename in core would silently blank the control
        // without this guard.
        let labels: Vec<&str> = MOTION_PREFERENCES.iter().map(|p| p.label()).collect();
        assert_eq!(MOTION_MODES.to_vec(), labels);
    }
}
