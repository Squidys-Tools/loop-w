//! Text-free window-preview renderer (iced canvas).
//!
//! Draws a rounded glass rect with the persisted padding, corner radius,
//! border width, and border color — the same surface used by the live
//! preview overlay.

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};

use crate::settings::color::to_iced;

/// Canvas program for one preview surface.
#[derive(Debug, Clone)]
pub struct PreviewCanvas {
    pub padding: f32,
    pub corner_radius: f32,
    pub border_width: f32,
    pub border: Color,
}

impl PreviewCanvas {
    pub fn from_settings(settings: &crate::settings::AppSettings) -> Self {
        Self {
            padding: settings.preview_padding as f32,
            corner_radius: settings.preview_corner_radius as f32,
            border_width: settings.preview_border_width as f32,
            border: to_iced(&settings.preview_border_color)
                .unwrap_or(Color::from_rgba(0.24, 0.61, 1.0, 0.72)),
        }
    }
}

impl<Message> canvas::Program<Message> for PreviewCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let pad = self.padding;
        let rect = Rectangle {
            x: pad,
            y: pad,
            width: (bounds.width - pad * 2.0).max(8.0),
            height: (bounds.height - pad * 2.0).max(8.0),
        };
        let rounded = Path::rounded_rectangle(
            Point::new(rect.x, rect.y),
            iced::Size::new(rect.width, rect.height),
            self.corner_radius.into(),
        );
        frame.fill(&rounded, Color::from_rgba(0.13, 0.16, 0.22, 0.55));
        if self.border_width > 0.01 {
            frame.stroke(
                &rounded,
                Stroke::default()
                    .with_width(self.border_width)
                    .with_color(self.border),
            );
        }
        vec![frame.into_geometry()]
    }
}

/// View helper for the preview surface.
///
/// The canvas fills the window's actual client area instead of using a fixed
/// size: `window::resize`/`move_to` apply asynchronously, so a fixed canvas
/// sized from the latest session state can overflow the real window by a
/// pixel after a move and get its edge clipped. `draw` already lays out from
/// `bounds.size()`, so filling always fits with no cutoff.
pub fn view<Message>(canvas: PreviewCanvas, _width: f32, _height: f32) -> Element<'static, Message>
where
    Message: Clone + Send + 'static,
{
    Canvas::new(canvas)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
