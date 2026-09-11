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
    /// Target rectangle in canvas coordinates. `None` draws the whole
    /// surface, which is used by the settings-page preview.
    pub target: Option<Rectangle>,
}

/// Persistent state owned by the Canvas widget tree.
///
/// The application rebuilds `PreviewCanvas` values whenever iced rebuilds a
/// window view. Keeping the cache in the Canvas state lets the renderer reuse
/// geometry across those view rebuilds instead of allocating a new frame for
/// every redraw request. The key is synchronized from `Program::update` so a
/// moved target or edited style still invalidates the cached geometry.
#[derive(Debug, Default)]
pub struct PreviewCanvasState {
    cache: canvas::Cache,
    key: Option<RenderKey>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RenderKey {
    padding: f32,
    corner_radius: f32,
    border_width: f32,
    border: Color,
    target: Option<Rectangle>,
}

impl PreviewCanvas {
    pub fn from_settings(settings: &crate::settings::AppSettings) -> Self {
        Self {
            padding: settings.preview_padding as f32,
            corner_radius: settings.preview_corner_radius as f32,
            border_width: settings.preview_border_width as f32,
            border: to_iced(&settings.preview_border_color)
                .unwrap_or(Color::from_rgba(0.24, 0.61, 1.0, 0.72)),
            target: None,
        }
    }
}

impl<Message> canvas::Program<Message> for PreviewCanvas {
    type State = PreviewCanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        _event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let key = self.render_key();
        if state.key != Some(key) {
            state.cache.clear();
            state.key = Some(key);
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = state.cache.draw(renderer, bounds.size(), |frame| {
            self.draw_geometry(frame, bounds);
        });

        vec![geometry]
    }
}

impl PreviewCanvas {
    fn render_key(&self) -> RenderKey {
        RenderKey {
            padding: self.padding,
            corner_radius: self.corner_radius,
            border_width: self.border_width,
            border: self.border,
            target: self.target,
        }
    }

    fn draw_geometry(&self, frame: &mut Frame, bounds: Rectangle) {
        let target = self.target.unwrap_or(Rectangle {
            x: 0.0,
            y: 0.0,
            width: bounds.width,
            height: bounds.height,
        });
        let left = target.x.max(0.0).min(bounds.width);
        let top = target.y.max(0.0).min(bounds.height);
        let right = (target.x + target.width).max(left).min(bounds.width);
        let bottom = (target.y + target.height).max(top).min(bounds.height);
        let pad = self.padding;
        let rect = Rectangle {
            x: left + pad,
            y: top + pad,
            width: (right - left - pad * 2.0).max(8.0),
            height: (bottom - top - pad * 2.0).max(8.0),
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
    }
}

/// View helper for the preview surface.
///
/// The canvas gets a concrete size: the settings page passes a fixed preview
/// size (a `Fill` canvas inside the settings `scrollable` resolves to a
/// degenerate height and rasterizes as a sliver — see BUGS.md #9), while the
/// live overlay passes its exact window size so the fixed canvas still fills
/// the surface. The live target stays a rectangle inside that surface, so
/// hover changes never resize the native window.
pub fn view<Message>(canvas: PreviewCanvas, width: f32, height: f32) -> Element<'static, Message>
where
    Message: Clone + Send + 'static,
{
    Canvas::new(canvas)
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AppSettings;

    #[test]
    fn render_key_invalidates_when_preview_target_moves() {
        let mut canvas = PreviewCanvas::from_settings(&AppSettings::default());
        let initial = canvas.render_key();

        canvas.target = Some(Rectangle {
            x: 8.0,
            y: 12.0,
            width: 320.0,
            height: 200.0,
        });

        assert_ne!(initial, canvas.render_key());
    }

    #[test]
    fn render_key_invalidates_when_preview_style_changes() {
        let mut canvas = PreviewCanvas::from_settings(&AppSettings::default());
        let initial = canvas.render_key();

        canvas.corner_radius += 1.0;

        assert_ne!(initial, canvas.render_key());
    }
}
