use iced::theme::Base;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{window, Element, Length, Rectangle, Theme};

use crate::win;

use super::{Message, Section, State};

pub(super) fn view(state: &State, window: window::Id) -> Element<'_, Message> {
    if Some(window) == state.main_id {
        return settings_view(state);
    }
    if let Some(session) = &state.radial {
        if session.id == window {
            let mut canvas =
                crate::ui::widgets::radial_canvas::RadialCanvas::from_settings(&state.settings);
            // Canvas shares the window's logical space, not physical pixels.
            let scale = session.scale as f32;
            canvas.outer_radius = session.outer as f32 / scale;
            canvas.inner_radius = session.inner as f32 / scale;
            canvas.hovered = session.hovered;
            return crate::ui::views::overlay::radial(canvas, session.window_size.0);
        }
    }
    if let Some(session) = &state.preview {
        if session.id == window {
            let mut canvas =
                crate::ui::widgets::preview_canvas::PreviewCanvas::from_settings(&state.settings);
            // Canvas shares the window's logical space, not physical pixels
            // (same scaling as the radial overlay above): without this the
            // padding, corner radius, and border grow with the DPI scale.
            let scale = session.scale as f32;
            if scale > 0.0 && scale.is_finite() {
                canvas.padding /= scale;
                canvas.corner_radius /= scale;
                canvas.border_width /= scale;
            }
            let (x, y, width, height) =
                win::overlay::to_local_logical(session.frame, session.overlay_frame, session.scale);
            canvas.target = Some(Rectangle {
                x,
                y,
                width,
                height,
            });
            return crate::ui::views::overlay::preview(
                canvas,
                session.window_size.0,
                session.window_size.1,
            );
        }
    }
    if state.tray_menu == Some(window) {
        return crate::ui::views::tray::view();
    }
    container(text("")).into()
}

fn settings_view(state: &State) -> Element<'_, Message> {
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
        Section::General => crate::ui::views::general::view(state),
        Section::Radial => crate::ui::views::radial::view(state),
        Section::Preview => crate::ui::views::preview::view(state),
        Section::Appearance => crate::ui::views::appearance::view(state),
        Section::Advanced => crate::ui::views::advanced::view(state),
    };

    let layout = row![
        container(sidebar.spacing(6).padding(16)).width(Length::Fixed(240.0)),
        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill),
    ];

    container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        // Opaque settings background, painted explicitly: the daemon-wide
        // clear color is transparent (for the overlay windows), so this
        // opaque window must fill its own surface. Same color the clear
        // used to provide (`Base::base`), so the look is unchanged.
        .style(|theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(Base::base(theme).background_color)),
            ..Default::default()
        })
        .into()
}
