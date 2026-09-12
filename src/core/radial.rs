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
    SlotGeometry {
        label: "Right",
        from_deg: -22.5,
        to_deg: 22.5,
    },
    SlotGeometry {
        label: "Bottom-right",
        from_deg: 22.5,
        to_deg: 67.5,
    },
    SlotGeometry {
        label: "Bottom",
        from_deg: 67.5,
        to_deg: 112.5,
    },
    SlotGeometry {
        label: "Bottom-left",
        from_deg: 112.5,
        to_deg: 157.5,
    },
    SlotGeometry {
        label: "Left",
        from_deg: 157.5,
        to_deg: 202.5,
    },
    SlotGeometry {
        label: "Top-left",
        from_deg: 202.5,
        to_deg: 247.5,
    },
    SlotGeometry {
        label: "Top",
        from_deg: 247.5,
        to_deg: 292.5,
    },
    SlotGeometry {
        label: "Top-right",
        from_deg: 292.5,
        to_deg: 337.5,
    },
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

/// Angular position (degrees) of a wedge boundary ray.
///
/// Separator lines belong on boundaries, not wedge centers: the highlight
/// for slot `i` fills `from_deg..to_deg`, so its edges sit exactly on
/// `boundary_deg(i)` and `boundary_deg(i + 1)`. Hit-testing
/// ([`index_at`]) switches wedges on these same rays.
pub fn boundary_deg(index: usize) -> f64 {
    GEOMETRY[index % GEOMETRY.len()].from_deg
}

/// Map a cursor angle (degrees, 0 = right, clockwise positive) to a wedge.
pub fn index_at(angle_deg: f64) -> usize {
    let normalized = angle_deg.rem_euclid(360.0);
    ((normalized + 22.5).div_euclid(45.0) as usize) % GEOMETRY.len()
}

/// Map a direction vector directly to a wedge without calculating an angle.
///
/// Runtime cursor handling can use this when it already has a vector from the
/// menu center. It avoids both `atan2` and degree conversion while preserving
/// the same boundary ownership as [`index_at`].
pub fn index_at_vector(dx: f64, dy: f64) -> usize {
    const TAN_22_5: f64 = 0.414_213_562_373_095_03;
    const COT_22_5: f64 = 2.414_213_562_373_095;

    if dx == 0.0 && dy == 0.0 {
        return 0;
    }

    if dx >= 0.0 {
        if dy >= 0.0 {
            if dy < dx * TAN_22_5 {
                0
            } else if dy < dx * COT_22_5 {
                1
            } else {
                2
            }
        } else if -dy < dx * TAN_22_5 {
            0
        } else if -dy < dx * COT_22_5 {
            7
        } else {
            6
        }
    } else if dy >= 0.0 {
        let left = -dx;
        if dy < left * TAN_22_5 {
            4
        } else if dy < left * COT_22_5 {
            3
        } else {
            2
        }
    } else {
        let left = -dx;
        if -dy < left * TAN_22_5 {
            4
        } else if -dy < left * COT_22_5 {
            5
        } else {
            6
        }
    }
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

    #[test]
    fn wedge_spans_tile_without_gaps() {
        let pairs = GEOMETRY
            .iter()
            .zip(GEOMETRY.iter().cycle().skip(1))
            .take(GEOMETRY.len());
        for (slot, next) in pairs {
            let gap = (slot.to_deg - next.from_deg).rem_euclid(360.0);
            assert!(gap < 1e-9, "gap of {gap}° after {}", slot.label);
        }
    }

    #[test]
    fn boundaries_match_hit_testing() {
        // Each boundary ray is exactly where `index_at` switches wedges,
        // so separator lines drawn on boundaries stay inline with the
        // committed sections.
        for (i, _) in GEOMETRY.iter().enumerate() {
            let b = boundary_deg(i);
            assert_eq!(index_at(b - 0.1), (i + GEOMETRY.len() - 1) % GEOMETRY.len());
            assert_eq!(index_at(b + 0.1), i % GEOMETRY.len());
        }
    }

    #[test]
    fn vector_hit_testing_matches_angle_hit_testing() {
        let directions = [
            (1.0, 0.0),
            (1.0, 1.0),
            (0.0, 1.0),
            (-1.0, 1.0),
            (-1.0, 0.0),
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
        ];

        for (index, (dx, dy)) in directions.into_iter().enumerate() {
            assert_eq!(index_at_vector(dx, dy), index);
            assert_eq!(index_at(angle_of(dx, dy)), index);
        }
    }

    #[test]
    fn normalizes_angles_without_overflowing_or_losing_negative_turns() {
        assert_eq!(index_at(-360.0 * 1_000_000.0 + 10.0), 0);
        assert_eq!(index_at(360.0 * 1_000_000.0 + 10.0), 0);
        assert_eq!(index_at(-90.0), 6);
    }
}
