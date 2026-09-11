//! Text-free radial menu renderer (iced canvas).
//!
//! Draws a clean donut — an annulus with a punched, see-through center
//! and no markings at rest. The only visible structure is the hovered
//! wedge's sector fill/stroke, so users perceive the divisions purely
//! through the highlight.

use iced::mouse;
use iced::widget::canvas::{self, Canvas, Fill, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};
use std::cell::Cell;
use std::sync::OnceLock;

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

/// Persistent canvas state retained by iced across parent view rebuilds.
/// Frame ticks can therefore reuse the ring until its inputs change and the
/// highlight until its inputs change, instead of rebuilding both every time.
#[derive(Debug)]
pub struct RadialCanvasState {
    ring_cache: canvas::Cache,
    highlight_cache: canvas::Cache,
    ring_key: Cell<Option<RingKey>>,
    highlight_key: Cell<Option<HighlightKey>>,
}

impl Default for RadialCanvasState {
    fn default() -> Self {
        Self {
            ring_cache: canvas::Cache::default(),
            highlight_cache: canvas::Cache::default(),
            ring_key: Cell::new(None),
            highlight_key: Cell::new(None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RingKey {
    outer_radius: u32,
    inner_radius: u32,
    ring: [u32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HighlightKey {
    outer_radius: u32,
    inner_radius: u32,
    hovered: Option<usize>,
    sector_fill: [u32; 4],
    sector_stroke: [u32; 4],
}

impl RadialCanvas {
    fn ring_key(&self) -> RingKey {
        RingKey {
            outer_radius: self.outer_radius.to_bits(),
            inner_radius: self.inner_radius.to_bits(),
            ring: color_key(self.ring),
        }
    }

    fn highlight_key(&self) -> HighlightKey {
        HighlightKey {
            outer_radius: self.outer_radius.to_bits(),
            inner_radius: self.inner_radius.to_bits(),
            hovered: self.hovered,
            sector_fill: color_key(self.sector_fill),
            sector_stroke: color_key(self.sector_stroke),
        }
    }
}

fn color_key(color: Color) -> [u32; 4] {
    [
        color.r.to_bits(),
        color.g.to_bits(),
        color.b.to_bits(),
        color.a.to_bits(),
    ]
}

impl<Message> canvas::Program<Message> for RadialCanvas {
    type State = RadialCanvasState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let ring_key = self.ring_key();
        if state.ring_key.get() != Some(ring_key) {
            state.ring_cache.clear();
            state.ring_key.set(Some(ring_key));
        }

        let highlight_key = self.highlight_key();
        if state.highlight_key.get() != Some(highlight_key) {
            state.highlight_cache.clear();
            state.highlight_key.set(Some(highlight_key));
        }

        let size = bounds.size();
        let center = Point::new(size.width / 2.0, size.height / 2.0);
        let outer = self.outer_radius.min(size.width / 2.0 - 4.0);
        let inner = self.inner_radius.min(outer - 8.0);

        let ring_geometry = state.ring_cache.draw(renderer, size, |frame| {
            // `Canvas` translates the renderer to the widget origin before
            // calling the program, while `Cache::draw` creates a frame whose
            // coordinates start at (0, 0). Keep all geometry in that local
            // frame; using bounds.x/y here double-applies the widget offset.
            draw_ring(frame, center, outer, inner, self.ring);
        });

        let Some(index) = self.hovered.filter(|index| GEOMETRY.get(*index).is_some()) else {
            return vec![ring_geometry];
        };

        let highlight_geometry = state.highlight_cache.draw(renderer, size, |frame| {
            let wedge = wedge_path(center, outer, inner, index);
            frame.fill(&wedge, self.sector_fill);
            frame.stroke(
                &wedge,
                Stroke::default()
                    .with_width(2.0)
                    .with_color(self.sector_stroke),
            );
        });

        vec![ring_geometry, highlight_geometry]
    }
}

fn draw_ring(frame: &mut canvas::Frame, center: Point, outer: f32, inner: f32, color: Color) {
    // Donut backdrop: outer disc with the center punched out (even-odd fill),
    // so the hole shows whatever is behind the overlay. The hole stays a
    // live commit target (release inside it commits the center action) — only
    // its visuals are cut out.
    let ring = Path::new(|p| {
        p.circle(center, outer);
        p.circle(center, inner);
    });
    frame.fill(
        &ring,
        Fill {
            rule: canvas::fill::Rule::EvenOdd,
            ..Fill::from(color)
        },
    );
}

const WEDGE_STEPS: usize = 24;
const WEDGE_POINT_COUNT: usize = 2 * (WEDGE_STEPS + 1);

type UnitWedge = [[f32; 2]; WEDGE_POINT_COUNT];

static UNIT_WEDGES: OnceLock<[UnitWedge; GEOMETRY.len()]> = OnceLock::new();

fn unit_wedges() -> &'static [UnitWedge; GEOMETRY.len()] {
    UNIT_WEDGES.get_or_init(|| {
        std::array::from_fn(|index| {
            let slot = GEOMETRY[index];
            let from = slot.from_deg.to_radians();
            let to = slot.to_deg.to_radians();

            std::array::from_fn(|point| {
                let step = if point <= WEDGE_STEPS {
                    point
                } else {
                    2 * WEDGE_STEPS + 1 - point
                };
                let t = from + (to - from) * (step as f64 / WEDGE_STEPS as f64);
                [t.cos() as f32, t.sin() as f32]
            })
        })
    })
}

fn wedge_path(center: Point, outer: f32, inner: f32, index: usize) -> Path {
    let unit_points = &unit_wedges()[index];
    let mut builder = canvas::path::Builder::new();
    for (point, unit) in unit_points.iter().enumerate() {
        let radius = if point <= WEDGE_STEPS { outer } else { inner };
        let p = Point::new(center.x + radius * unit[0], center.y + radius * unit[1]);
        if point == 0 {
            builder.move_to(p);
        } else {
            builder.line_to(p);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> RadialCanvas {
        RadialCanvas {
            outer_radius: 96.0,
            inner_radius: 32.0,
            hovered: None,
            ring: Color::BLACK,
            sector_fill: Color::WHITE,
            sector_stroke: Color::WHITE,
        }
    }

    #[test]
    fn radial_geometry_uses_canvas_local_coordinates() {
        let layout_bounds = Rectangle {
            x: 256.0,
            y: 82.9,
            width: 260.0,
            height: 260.0,
        };
        let size = layout_bounds.size();

        assert_eq!(
            Point::new(size.width / 2.0, size.height / 2.0),
            Point::new(130.0, 130.0)
        );
    }

    #[test]
    fn hover_changes_only_highlight_key() {
        let mut canvas = canvas();
        let ring = canvas.ring_key();
        let highlight = canvas.highlight_key();

        canvas.hovered = Some(2);

        assert_eq!(ring, canvas.ring_key());
        assert_ne!(highlight, canvas.highlight_key());
    }

    #[test]
    fn ring_style_change_invalidates_ring_key() {
        let mut canvas = canvas();
        let initial = canvas.ring_key();

        canvas.ring = Color::from_rgb(0.2, 0.3, 0.4);

        assert_ne!(initial, canvas.ring_key());
    }
}
