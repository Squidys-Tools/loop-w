//! Stash record types shared by persistence and the stash service.

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::rect::{Point, Rect};

/// Edge a window is stashed against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum StashEdge {
    Left = 0,
    Right = 1,
    Top = 2,
    Bottom = 3,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StashPoint {
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
}

impl From<Point> for StashPoint {
    fn from(p: Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<StashPoint> for Point {
    fn from(p: StashPoint) -> Self {
        Point::new(p.x, p.y)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StashRect {
    #[serde(default)]
    pub left: i32,
    #[serde(default)]
    pub top: i32,
    #[serde(default)]
    pub right: i32,
    #[serde(default)]
    pub bottom: i32,
}

impl From<Rect> for StashRect {
    fn from(r: Rect) -> Self {
        Self { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
    }
}

impl From<StashRect> for Rect {
    fn from(r: StashRect) -> Self {
        Rect::new(r.left, r.top, r.right, r.bottom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StashPlacement {
    #[serde(default)]
    pub length: i32,
    #[serde(default)]
    pub flags: u32,
    #[serde(default)]
    pub show_command: u32,
    #[serde(default)]
    pub min_position: StashPoint,
    #[serde(default)]
    pub max_position: StashPoint,
    #[serde(default)]
    pub normal_position: StashRect,
}

impl Default for StashPlacement {
    fn default() -> Self {
        Self {
            length: 0,
            flags: 0,
            show_command: 0,
            min_position: StashPoint { x: 0, y: 0 },
            max_position: StashPoint { x: 0, y: 0 },
            normal_position: StashRect { left: 0, top: 0, right: 0, bottom: 0 },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StashMonitor {
    #[serde(default)]
    pub monitor: StashRect,
    #[serde(default)]
    pub work: StashRect,
    #[serde(default = "default_dpi")]
    pub dpi_x: f64,
    #[serde(default = "default_dpi")]
    pub dpi_y: f64,
}

fn default_dpi() -> f64 {
    96.0
}

impl Default for StashMonitor {
    fn default() -> Self {
        Self {
            monitor: StashRect { left: 0, top: 0, right: 0, bottom: 0 },
            work: StashRect { left: 0, top: 0, right: 0, bottom: 0 },
            dpi_x: 96.0,
            dpi_y: 96.0,
        }
    }
}

/// One stashed window plus the metadata needed to restore it safely.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StashRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub executable_path: String,
    #[serde(default)]
    pub process_id: u32,
    #[serde(default)]
    pub window_class: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub edge: StashEdge,
    #[serde(default)]
    pub original_placement: StashPlacement,
    #[serde(default)]
    pub original_monitor: StashMonitor,
    #[serde(default)]
    pub stashed_frame: StashRect,
}

impl Default for StashEdge {
    fn default() -> Self {
        StashEdge::Left
    }
}

/// Edge of the work area a window is nearest to. Strict `<` scan in
/// Left, Right, Top, Bottom order, so ties break toward Left.
pub fn nearest_edge(work: Rect, window: Rect) -> StashEdge {
    let left = (window.left - work.left).abs();
    let right = (work.right - window.right).abs();
    let top = (window.top - work.top).abs();
    let bottom = (work.bottom - window.bottom).abs();
    let mut edge = StashEdge::Left;
    let mut best = left;
    if right < best {
        edge = StashEdge::Right;
        best = right;
    }
    if top < best {
        edge = StashEdge::Top;
        best = top;
    }
    if bottom < best {
        edge = StashEdge::Bottom;
        best = bottom;
    }
    edge
}

/// Stashed frame: the window keeps its full size and cross-axis position;
/// only the stash axis moves off-screen, leaving `peek` pixels visible.
pub fn calculate_stashed_frame(work: Rect, window: Rect, edge: StashEdge, peek: i32) -> Rect {
    let width = window.width().max(1);
    let height = window.height().max(1);
    let visible = peek.clamp(1, 48).max(1).min(width.min(height));
    match edge {
        StashEdge::Left => Rect::new(
            work.left - width + visible,
            window.top,
            work.left + visible,
            window.bottom,
        ),
        StashEdge::Right => Rect::new(
            work.right - visible,
            window.top,
            work.right + width - visible,
            window.bottom,
        ),
        StashEdge::Top => Rect::new(
            window.left,
            work.top - height + visible,
            window.right,
            work.top + visible,
        ),
        StashEdge::Bottom => Rect::new(
            window.left,
            work.bottom - visible,
            window.right,
            work.bottom + height - visible,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stash_frames_keep_visible_peek() {
        let work = Rect::new(0, 0, 1000, 800);
        let window = Rect::new(100, 120, 500, 520);
        assert_eq!(nearest_edge(work, Rect::new(-2, 120, 398, 520)), StashEdge::Left);
        assert_eq!(
            calculate_stashed_frame(work, window, StashEdge::Left, 8),
            Rect::new(-392, 120, 8, 520)
        );
        assert_eq!(
            calculate_stashed_frame(work, window, StashEdge::Right, 8),
            Rect::new(992, 120, 1392, 520)
        );
    }
}
