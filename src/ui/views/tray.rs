//! Dark tray popup rendered by iced instead of the native Windows menu.

use iced::widget::{button, column, container, text};
use iced::{Background, Color, Element, Length, Theme};

use crate::ui::app::Message;

pub fn view() -> Element<'static, Message> {
    let content = column![
        text("LoopW").size(16),
        button(text("Open settings").size(14))
            .width(Length::Fill)
            .on_press(Message::TrayMenuSettings),
        button(text("Quit").size(14))
            .width(Length::Fill)
            .on_press(Message::TrayMenuQuit),
    ]
    .spacing(6)
    .padding(12);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgb8(27, 33, 43))),
            text_color: Some(Color::from_rgb8(235, 240, 248)),
            border: iced::Border {
                color: Color::from_rgb8(71, 84, 104),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}
