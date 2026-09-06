//! Radial menu slot geometry: eight 45° wedges, no text in the overlay.
//!
//! Slot 0 points right (-22.5°..22.5°) and indices advance clockwise in
//! screen coordinates (y down), matching the C# `RadialActionCatalog`.

use super::actions::WindowAction;

/// One wedge's angular span in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotGeometry {
    pub label: &'static str,
    pub from_deg: f64,
    pub to_deg: f64,
}

impl SlotGeometry {
    pub const fn center_deg(self) -> f64 {
        (self.from_deg + self.to_deg) / 2.0
    }
}

/// Clockwise wedge layout starting at Right.
pub const GEOMETRY: [SlotGeometry; 8] = [
    SlotGeometry { label: "Right", from_deg: -22.5, to_deg: 22.5 },
    SlotGeometry { label: "Bottom-right", from_deg: 22.5, to_deg: 67.5 },
    SlotGeometry { label: "Bottom", from_deg: 67.5, to_deg: 112.5 },
    SlotGeometry { label: "Bottom-left", from_deg: 112.5, to_deg: 157.5 },
    SlotGeometry { label: "Left", from_deg: 157.5, to_deg: 202.5 },
    SlotGeometry { label: "Top-left", from_deg: 202.5, to_deg: 247.5 },
    SlotGeometry { label: "Top", from_deg: 247.5, to_deg: 292.5 },
    SlotGeometry { label: "Top-right", from_deg: 292.5, to_deg: 337.5 },
];

/// Default actions per wedge (kept for compat/tests).
pub const DEFAULT_ACTIONS: [WindowAction; 8] = [
    WindowAction::RightHalf,
    WindowAction::BottomRightQuarter,
    WindowAction::BottomHalf,
    WindowAction::BottomLeftQuarter,
    WindowAction::LeftHalf,
    WindowAction::TopLeftQuarter,
    WindowAction::TopHalf,
    WindowAction::TopRightQuarter,
];

pub const SLOT_COUNT: usize = 8;

/// Map a cursor angle (degrees, 0 = right, clockwise positive) to a wedge.
pub fn index_at(angle_deg: f64) -> usize {
    let normalized = (angle_deg + 360.0) % 360.0;
    ((normalized + 22.5).div_euclid(45.0) as usize) % GEOMETRY.len()
}

/// Default action for a cursor angle.
pub fn action_at(angle_deg: f64) -> WindowAction {
    DEFAULT_ACTIONS[index_at(angle_deg)]
}

/// Angle of a point relative to a center, in the same convention.
pub fn angle_of(dx: f64, dy: f64) -> f64 {
    dy.atan2(dx).to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_octant() {
        assert_eq!(action_at(0.0), WindowAction::RightHalf);
        assert_eq!(action_at(45.0), WindowAction::BottomRightQuarter);
        assert_eq!(action_at(90.0), WindowAction::BottomHalf);
        assert_eq!(action_at(135.0), WindowAction::BottomLeftQuarter);
        assert_eq!(action_at(180.0), WindowAction::LeftHalf);
        assert_eq!(action_at(225.0), WindowAction::TopLeftQuarter);
        assert_eq!(action_at(270.0), WindowAction::TopHalf);
        assert_eq!(action_at(315.0), WindowAction::TopRightQuarter);
        assert_eq!(action_at(-90.0), WindowAction::TopHalf);
    }

    #[test]
    fn eight_unique_defaults() {
        let mut distinct = DEFAULT_ACTIONS.to_vec();
        distinct.sort_by_key(|a| *a as u8);
        distinct.dedup();
        assert_eq!(distinct.len(), 8);
    }
}
