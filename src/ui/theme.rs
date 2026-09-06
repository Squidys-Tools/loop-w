//! Curated appearance presets + theme tokens.
//!
//! Presets apply immediately and save; diverging from a preset surfaces a
//! "Custom" state while keeping raw color editing available underneath.

use serde::{Deserialize, Serialize};

/// Named preset driving accent + overlay colors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preset {
    pub name: &'static str,
    pub accent: &'static str,
    pub sector_fill: &'static str,
    pub sector_stroke: &'static str,
    pub ring_fill: &'static str,
    pub preview_border: &'static str,
}

pub const PRESETS: [Preset; 3] = [
    Preset {
        name: "LoopW Blue",
        accent: "#3D9BFF",
        sector_fill: "#7A3D9BFF",
        sector_stroke: "#F03D9BFF",
        ring_fill: "#B61B212B",
        preview_border: "#B83D9BFF",
    },
    Preset {
        name: "LoopW Cobalt",
        accent: "#2F6BFF",
        sector_fill: "#7A2F6BFF",
        sector_stroke: "#F02F6BFF",
        ring_fill: "#B6161D2B",
        preview_border: "#B82F6BFF",
    },
    Preset {
        name: "LoopW Violet",
        accent: "#8B5CF6",
        sector_fill: "#7A8B5CF6",
        sector_stroke: "#F08B5CF6",
        ring_fill: "#B61E1B2E",
        preview_border: "#B88B5CF6",
    },
];

/// Name of the preset matching the current colors, if any.
pub fn matching_preset(
    accent: &str,
    sector_fill: &str,
    sector_stroke: &str,
    ring_fill: &str,
    preview_border: &str,
) -> Option<&'static str> {
    use crate::settings::normalize_color;
    let (a, sf, ss, rf, pb) = (
        normalize_color(accent, "#000000"),
        normalize_color(sector_fill, "#000000"),
        normalize_color(sector_stroke, "#000000"),
        normalize_color(ring_fill, "#000000"),
        normalize_color(preview_border, "#000000"),
    );
    PRESETS.iter().find(|p| {
        normalize_color(p.accent, "#111111") == a
            && normalize_color(p.sector_fill, "#111111") == sf
            && normalize_color(p.sector_stroke, "#111111") == ss
            && normalize_color(p.ring_fill, "#111111") == rf
            && normalize_color(p.preview_border, "#111111") == pb
    }).map(|p| p.name)
}
