//! `#RRGGBB` / `#AARRGGBB` color normalization.
//!
//! Accepts the same inputs as the WPF `ColorConverter` path used by the C#
//! app and always returns an uppercase `#AARRGGBB` string so overlay code has
//! one canonical form. Invalid values fall back without breaking the file.

/// Normalize a color string, returning `fallback` when invalid.
pub fn normalize_color(value: &str, fallback: &str) -> String {
    parse_color(value)
        .unwrap_or_else(|| parse_color(fallback).unwrap_or_else(|| "#FF007AFF".to_string()))
}

fn parse_color(value: &str) -> Option<String> {
    let hex = value.trim().trim_start_matches('#');
    match hex.len() {
        6 => {
            let rgb = u32::from_str_radix(hex, 16).ok()?;
            Some(format!("#FF{rgb:06X}"))
        }
        8 => {
            let argb = u32::from_str_radix(hex, 16).ok()?;
            Some(format!("#{argb:08X}"))
        }
        _ => None,
    }
}

/// Split a canonical `#AARRGGBB` color into `[a, r, g, b]` bytes.
pub fn to_argb_bytes(canonical: &str) -> Option<[u8; 4]> {
    let hex = canonical.trim().trim_start_matches('#');
    if hex.len() != 8 {
        return None;
    }
    let argb = u32::from_str_radix(hex, 16).ok()?;
    Some([
        ((argb >> 24) & 0xFF) as u8,
        ((argb >> 16) & 0xFF) as u8,
        ((argb >> 8) & 0xFF) as u8,
        (argb & 0xFF) as u8,
    ])
}

/// Convert a canonical color to iced (sRGB 0..1 floats).
pub fn to_iced(canonical: &str) -> Option<iced::Color> {
    let [a, r, g, b] = to_argb_bytes(canonical)?;
    Some(iced::Color::from_rgba(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_rgb_and_argb() {
        assert_eq!(normalize_color("#3D9BFF", "#FF007AFF"), "#FF3D9BFF");
        assert_eq!(normalize_color("#7A3D9BFF", "#FF007AFF"), "#7A3D9BFF");
    }

    #[test]
    fn falls_back_on_invalid() {
        assert_eq!(normalize_color("nope", "#FF007AFF"), "#FF007AFF");
        assert_eq!(normalize_color("", "#FF007AFF"), "#FF007AFF");
    }
}
