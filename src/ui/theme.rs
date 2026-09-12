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

pub const PRESET_NAMES: [&str; 3] = ["LoopW Blue", "LoopW Cobalt", "LoopW Violet"];

/// Name of the preset matching the current colors, if any.
pub fn matching_preset(
    accent: &str,
    sector_fill: &str,
    sector_stroke: &str,
    ring_fill: &str,
    preview_border: &str,
) -> Option<&'static str> {
    let colors = [
        parse_color(accent)?,
        parse_color(sector_fill)?,
        parse_color(sector_stroke)?,
        parse_color(ring_fill)?,
        parse_color(preview_border)?,
    ];

    PRESETS
        .iter()
        .find(|preset| {
            [
                parse_color(preset.accent),
                parse_color(preset.sector_fill),
                parse_color(preset.sector_stroke),
                parse_color(preset.ring_fill),
                parse_color(preset.preview_border),
            ]
            .into_iter()
            .flatten()
            .eq(colors)
        })
        .map(|preset| preset.name)
}

fn parse_color(value: &str) -> Option<[u8; 4]> {
    let hex = value.trim().trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).ok()?;

    match hex.len() {
        6 => Some([
            0xFF,
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        ]),
        8 => Some([
            ((value >> 24) & 0xFF) as u8,
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        ]),
        _ => None,
    }
}
