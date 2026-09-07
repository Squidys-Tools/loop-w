//! Human-readable hotkey names (`Ctrl + Shift + A`, `Caps Lock`, ...).
//!
//! Ports `HotkeyNames.For` including left/right modifier-side labels.

use serde_repr::{Deserialize_repr, Serialize_repr};

/// Which physical side of a modifier activates the trigger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum TriggerModifierSide {
    #[default]
    Any = 0,
    Left = 1,
    Right = 2,
}

/// All valid modifier-side values (for normalization).
pub const TRIGGER_MODIFIER_SIDES: [TriggerModifierSide; 3] = [
    TriggerModifierSide::Any,
    TriggerModifierSide::Left,
    TriggerModifierSide::Right,
];

pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;

pub const VK_CAPITAL: u32 = 0x14;
pub const VK_SPACE: u32 = 0x20;
pub const VK_ESCAPE: u32 = 0x1B;

pub fn hotkey_name(modifiers: u32, vk: u32, side: TriggerModifierSide) -> String {
    let side_label = match side {
        TriggerModifierSide::Left => "Left",
        TriggerModifierSide::Right => "Right",
        TriggerModifierSide::Any => "",
    };
    let mut parts = Vec::with_capacity(4);
    let prefixed = |base: &str| {
        if side_label.is_empty() {
            base.to_string()
        } else {
            format!("{side_label} {base}")
        }
    };
    if modifiers & MOD_CONTROL != 0 {
        parts.push(prefixed("Ctrl"));
    }
    if modifiers & MOD_ALT != 0 {
        parts.push(prefixed("Alt"));
    }
    if modifiers & MOD_SHIFT != 0 {
        parts.push(prefixed("Shift"));
    }
    if modifiers & MOD_WIN != 0 {
        parts.push(prefixed("Win"));
    }
    parts.push(key_name(vk));
    parts.join(" + ")
}

pub fn key_name(vk: u32) -> String {
    match vk {
        0x08 => "Backspace".to_string(),
        0x09 => "Tab".to_string(),
        0x0D => "Enter".to_string(),
        0x13 => "Pause".to_string(),
        0x1B => "Esc".to_string(),
        0x14 => "Caps Lock".to_string(),
        0x20 => "Space".to_string(),
        0x21 => "PgUp".to_string(),
        0x22 => "PgDn".to_string(),
        0x23 => "End".to_string(),
        0x24 => "Home".to_string(),
        0x25 => "Left".to_string(),
        0x26 => "Up".to_string(),
        0x27 => "Right".to_string(),
        0x28 => "Down".to_string(),
        0x2C => "PrtScr".to_string(),
        0x2D => "Ins".to_string(),
        0x2E => "Del".to_string(),
        0x5D => "Menu".to_string(),
        0x90 => "Num Lock".to_string(),
        0x91 => "Scroll Lock".to_string(),
        0x30..=0x39 => char::from_u32(vk)
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("VK{vk:02X}")),
        0x41..=0x5A => char::from_u32(vk)
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("VK{vk:02X}")),
        0x60..=0x69 => format!("Num {}", vk - 0x60),
        0x6A => "Num *".to_string(),
        0x6B => "Num +".to_string(),
        0x6D => "Num -".to_string(),
        0x6E => "Num .".to_string(),
        0x6F => "Num /".to_string(),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        0xBA => ";".to_string(),
        0xBB => "=".to_string(),
        0xBC => ",".to_string(),
        0xBD => "-".to_string(),
        0xBE => ".".to_string(),
        0xBF => "/".to_string(),
        0xC0 => "`".to_string(),
        0xDB => "[".to_string(),
        0xDC => "\\".to_string(),
        0xDD => "]".to_string(),
        0xDE => "'".to_string(),
        _ => format!("VK{vk:02X}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_modifier_side() {
        assert_eq!(
            hotkey_name(MOD_CONTROL, 0x42, TriggerModifierSide::Right),
            "Right Ctrl + B"
        );
        assert_eq!(
            hotkey_name(MOD_CONTROL, 0x42, TriggerModifierSide::Left),
            "Left Ctrl + B"
        );
        assert_eq!(
            hotkey_name(MOD_SHIFT, VK_CAPITAL, TriggerModifierSide::Any),
            "Shift + Caps Lock"
        );
    }
}
