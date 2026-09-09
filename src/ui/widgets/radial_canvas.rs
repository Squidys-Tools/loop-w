//! Text-free radial menu renderer (iced canvas).
//!
//! Draws a clean donut — an annulus with a punched, see-through center
//! and no markings at rest. The only visible structure is the hovered
//! wedge's sector fill/stroke, so users perceive the divisions purely
//! through the highlight.

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Fill, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};

use crate::core::radial::GEOMETRY;
use crate::settings::color::to_iced;

/// Canvas program state for one radial surface.
#[derive(Debug, Clone, Default)]
pub struct RadialCanvas {
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub hovered: Option<usize>,
    pub ring: Color,
    pub sector_fill: Color,
    pub sector_stroke: Color,
}

impl RadialCanvas {
    pub fn from_settings(settings: &crate::settings::AppSettings) -> Self {
        Self {
            outer_radius: settings.radial_outer_radius as f32,
            inner_radius: settings.radial_inner_radius as f32,
            hovered: None,
            ring: to_iced(&settings.radial_ring_fill)
                .unwrap_or(Color::from_rgba(0.1, 0.13, 0.17, 0.71)),
            sector_fill: to_iced(&settings.radial_sector_fill)
                .unwrap_or(Color::from_rgba(0.24, 0.61, 1.0, 0.48)),
            sector_stroke: to_iced(&settings.radial_sector_stroke)
                .unwrap_or(Color::from_rgba(0.24, 0.61, 1.0, 0.94)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum RadialCanvasMessage {
    Hover(Option<usize>),
}

impl<Message> canvas::Program<Message> for RadialCanvas {
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
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let outer = self.outer_radius.min(bounds.width / 2.0 - 4.0);
        let inner = self.inner_radius.min(outer - 8.0);

        // Donut backdrop: outer disc with the center punched out
        // (even-odd fill), so the hole shows whatever is behind the
        // overlay. The hole stays a live commit target (release inside it
        // commits the center action) — only its visuals are cut out.
        let ring = Path::new(|p| {
            p.circle(center, outer);
            p.circle(center, inner);
        });
        frame.fill(
            &ring,
            Fill {
                rule: canvas::fill::Rule::EvenOdd,
                ..Fill::from(self.ring)
            },
        );
        // Hovered wedge highlight only — this is the single visual that
        // reveals the wedge divisions.
        if let Some(index) = self.hovered {
            if let Some(slot) = GEOMETRY.get(index) {
                let wedge = wedge_path(center, outer, inner, slot.from_deg, slot.to_deg);
                frame.fill(&wedge, self.sector_fill);
                frame.stroke(
                    &wedge,
                    Stroke::default()
                        .with_width(2.0)
                        .with_color(self.sector_stroke),
                );
            }
        }

        vec![frame.into_geometry()]
    }
}

fn wedge_path(center: Point, outer: f32, inner: f32, from_deg: f64, to_deg: f64) -> Path {
    use std::f64::consts::PI;
    let steps = 24;
    let from = from_deg * PI / 180.0;
    let to = to_deg * PI / 180.0;
    let mut builder = canvas::path::Builder::new();
    for i in 0..=steps {
        let t = from + (to - from) * (i as f64 / steps as f64);
        let p = Point::new(
            center.x + outer * t.cos() as f32,
            center.y + outer * t.sin() as f32,
        );
        if i == 0 {
            builder.move_to(p);
        } else {
            builder.line_to(p);
        }
    }
    for i in (0..=steps).rev() {
        let t = from + (to - from) * (i as f64 / steps as f64);
        builder.line_to(Point::new(
            center.x + inner * t.cos() as f32,
            center.y + inner * t.sin() as f32,
        ));
    }
    builder.close();
    builder.build()
}

/// View helper: fixed-size radial preview with hover reporting.
pub fn view<Message>(canvas: RadialCanvas, size: f32) -> Element<'static, Message>
where
    Message: Clone + Send + 'static,
{
    Canvas::new(canvas)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}
