//! Overlay window views: radial menu + target preview.
//!
//! Both are text-free by invariant. The radial overlay keeps mouse clicks
//! (press commits the hovered wedge); the preview overlay is purely visual
//! and made click-through at the Win32 level after opening.

use iced::widget::{container, mouse_area};
use iced::{Element, Length};

use crate::ui::app::Message;
use crate::ui::widgets::preview_canvas::{self, PreviewCanvas};
use crate::ui::widgets::radial_canvas::{self, RadialCanvas};

/// Full-window radial surface. `size` is the square window edge.
pub fn radial(canvas: RadialCanvas, size: f32) -> Element<'static, Message> {
    mouse_area(radial_canvas::view(canvas, size))
        .on_press(Message::OverlayCommit)
        .into()
}

/// Full-window preview surface sized to the target frame.
pub fn preview(canvas: PreviewCanvas, width: f32, height: f32) -> Element<'static, Message> {
    container(preview_canvas::view(canvas, width, height))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
