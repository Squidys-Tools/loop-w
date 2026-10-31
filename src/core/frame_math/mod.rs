//! Window frame math: zones, axis maximize, fill, center, nudge, fit.
//!
//! Direct port of `WindowFrameMath`. All functions are pure and DPI aware
//! through explicit parameters so they stay unit testable.
//!
//! Split by concern: `zones` (grid placements), `axis` (single-axis
//! maximize), `fill` (obstacle-free area), `center`, `manipulate`
//! (grow/shrink/nudge steps) and `fit` (min/max limits + edge anchoring).
//! Everything is re-exported here so `crate::core::frame_math::*` paths are
//! unchanged for callers.

mod axis;
mod center;
mod fill;
mod fit;
mod manipulate;
mod zones;

pub use axis::{maximize_height_frame, maximize_width_frame};
pub use center::center_frame;
pub use fill::fill_available_frame;
pub use fit::{fit_frame, MinMaxLimits};
pub use manipulate::manipulate_frame;
pub use zones::zone_frame;
