//! Pure keybind matching: first list entry wins, exact modifiers.
//!
//! Ports the `TryMatchKeybindLocked` scan so the matching rules are unit
//! tested without a hook. Repeat suppression (`_pressedKeybinds`) and the
//! trigger-held / timed-out gates live in `crate::win::hooks`.

/// Minimal keybind view for matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchEntry {
    pub vk: u32,
    pub modifiers: u32,
    pub bypass_trigger: bool,
}

/// First index whose vk matches, whose modifiers match exactly, whose
/// bypass kind matches the current trigger state, and which is not the
/// trigger key itself. Returns `None` when nothing matches.
pub fn match_keybind(
    entries: &[MatchEntry],
    vk: u32,
    modifiers: u32,
    trigger_vk: u32,
    trigger_held: bool,
) -> Option<usize> {
    entries.iter().position(|entry| {
        entry.vk == vk
            && entry.vk != trigger_vk
            && entry.modifiers == modifiers
            && entry.bypass_trigger == !trigger_held
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<MatchEntry> {
        vec![
            MatchEntry {
                vk: 0x41,
                modifiers: 0x0002,
                bypass_trigger: false,
            },
            MatchEntry {
                vk: 0x41,
                modifiers: 0x0006,
                bypass_trigger: false,
            },
            MatchEntry {
                vk: 0x42,
                modifiers: 0,
                bypass_trigger: true,
            },
        ]
    }

    #[test]
    fn normal_keybinds_need_trigger_and_exact_mods() {
        let list = entries();
        assert_eq!(match_keybind(&list, 0x41, 0x0002, 0x14, true), Some(0));
        // Extra modifiers veto the match.
        assert_eq!(match_keybind(&list, 0x41, 0x0003, 0x14, true), None);
        // Second entry matches its own exact combo.
        assert_eq!(match_keybind(&list, 0x41, 0x0006, 0x14, true), Some(1));
        // Normal keybinds never fire without the trigger.
        assert_eq!(match_keybind(&list, 0x41, 0x0002, 0x14, false), None);
    }

    #[test]
    fn bypass_keybinds_need_released_trigger() {
        let list = entries();
        assert_eq!(match_keybind(&list, 0x42, 0, 0x14, false), Some(2));
        assert_eq!(match_keybind(&list, 0x42, 0, 0x14, true), None);
    }

    #[test]
    fn trigger_key_itself_never_matches() {
        let list = entries();
        assert_eq!(match_keybind(&list, 0x14, 0, 0x14, true), None);
    }
}
