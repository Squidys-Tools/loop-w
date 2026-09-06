//! Preview section: shared glass surface first, geometry below.

use iced::widget::{button, column, container, row, slider, text, toggler};
use iced::{Element, Length};

use crate::ui::app::{Message, Section, State};
use crate::ui::widgets::preview_canvas::{self, PreviewCanvas};

pub fn view(state: &State) -> Element<'_, Message> {
    let settings = &state.settings;
    let canvas = PreviewCanvas::from_settings(settings);

    let content = column![
        text("Target preview").size(20),
        text("Shows where the window will land before you release.").size(13),
        preview_canvas::view(canvas, 320.0, 200.0),
        row![
            text("Preview enabled").size(14),
            toggler(settings.preview_enabled).on_toggle(Message::SetPreviewEnabled),
            text("Drag snapping").size(14),
            toggler(settings.drag_snap_enabled).on_toggle(Message::SetDragSnap),
        ]
        .spacing(12),
        text(format!("Padding: {:.0} px", settings.preview_padding)).size(14),
        slider(4.0..=48.0, settings.preview_padding as f32, Message::SetPreviewPadding),
        text(format!("Corner radius: {:.0} px", settings.preview_corner_radius)).size(14),
        slider(
            4.0..=32.0,
            settings.preview_corner_radius as f32,
            Message::SetPreviewRadius
        ),
        text(format!("Border width: {:.1} px", settings.preview_border_width)).size(14),
        slider(
            0.0..=6.0,
            settings.preview_border_width as f32,
            Message::SetPreviewBorder
        ),
        text(format!("Snap threshold: {} px", settings.drag_snap_threshold)).size(14),
        slider(
            4.0..=96.0,
            settings.drag_snap_threshold as f32,
            Message::SetSnapThreshold
        ),
        row![
            text("Restore frame on snap cancel").size(14),
            toggler(settings.restore_pre_drag_on_cancel)
                .on_toggle(Message::SetRestoreOnCancel),
        ]
        .spacing(12),
        row![
            button("Reset section").on_press(Message::ResetSection(Section::Preview)),
        ],
        text(&state.status).size(12),
    ]
    .spacing(12)
    .padding(16);

    container(content).width(Length::Fill).height(Length::Fill).into()
}
