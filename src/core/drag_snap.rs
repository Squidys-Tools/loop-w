//! Title-bar drag snapping: edges and corners resolve to zones.

use super::actions::WindowAction;
use super::rect::{Point, Rect};

/// Snap target for a drag near a monitor edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragSnapZone {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter,
}

impl DragSnapZone {
    pub fn action(self) -> WindowAction {
        match self {
            DragSnapZone::LeftHalf => WindowAction::LeftHalf,
            DragSnapZone::RightHalf => WindowAction::RightHalf,
            DragSnapZone::TopHalf => WindowAction::TopHalf,
            DragSnapZone::BottomHalf => WindowAction::BottomHalf,
            DragSnapZone::TopLeftQuarter => WindowAction::TopLeftQuarter,
            DragSnapZone::TopRightQuarter => WindowAction::TopRightQuarter,
            DragSnapZone::BottomLeftQuarter => WindowAction::BottomLeftQuarter,
            DragSnapZone::BottomRightQuarter => WindowAction::BottomRightQuarter,
        }
    }
}

/// Resolve a cursor position to a snap zone, if within `threshold` of an edge.
pub fn try_resolve(
    monitor: Rect,
    work_area: Rect,
    cursor: Point,
    threshold: i32,
) -> Option<DragSnapZone> {
    let threshold = threshold.clamp(1, 256);
    if monitor.is_empty()
        || work_area.is_empty()
        || cursor.x < monitor.left
        || cursor.x > monitor.right
        || cursor.y < monitor.top
        || cursor.y > monitor.bottom
    {
        return None;
    }
    let near_left = cursor.x - monitor.left <= threshold;
    let near_right = monitor.right - cursor.x <= threshold;
    let near_top = cursor.y - monitor.top <= threshold;
    let near_bottom = monitor.bottom - cursor.y <= threshold;
    if !(near_left || near_right || near_top || near_bottom) {
        return None;
    }
    Some(match (near_left, near_right, near_top, near_bottom) {
        (true, false, true, false) => DragSnapZone::TopLeftQuarter,
        (false, true, true, false) => DragSnapZone::TopRightQuarter,
        (true, false, false, true) => DragSnapZone::BottomLeftQuarter,
        (false, true, false, true) => DragSnapZone::BottomRightQuarter,
        (true, false, _, _) => DragSnapZone::LeftHalf,
        (false, true, _, _) => DragSnapZone::RightHalf,
        (_, _, true, false) => DragSnapZone::TopHalf,
        (_, _, false, true) => DragSnapZone::BottomHalf,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_edges_and_corners() {
        let monitor = Rect::new(0, 0, 1920, 1080);
        let work = Rect::new(0, 0, 1920, 1040);
        assert_eq!(
            try_resolve(monitor, work, Point::new(8, 8), 24),
            Some(DragSnapZone::TopLeftQuarter)
        );
        assert_eq!(
            try_resolve(monitor, work, Point::new(1912, 500), 24),
            Some(DragSnapZone::RightHalf)
        );
        assert_eq!(
            try_resolve(monitor, work, Point::new(900, 1072), 24),
            Some(DragSnapZone::BottomHalf)
        );
    }

    #[test]
    fn ignores_monitor_interior() {
        let monitor = Rect::new(0, 0, 1920, 1080);
        let work = Rect::new(0, 0, 1920, 1040);
        assert_eq!(
            try_resolve(monitor, work, Point::new(960, 500), 24),
            None
        );
    }
}
