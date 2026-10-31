use std::collections::HashSet;

use super::capture::{capture_decision, CaptureDecision, CaptureTarget};
use super::consts::VK_ESCAPE;
use super::state_machine::matched_keybind_index;
use super::{Config, HookState, KeybindSnapshot};
use crate::core::hotkey::{TriggerModifierSide, MOD_ALT, MOD_CONTROL, MOD_WIN};
use crate::core::keybind_match::MatchEntry;

fn state_with_keybinds(keybinds: Vec<KeybindSnapshot>) -> HookState {
    let match_entries = keybinds
        .iter()
        .map(|keybind| MatchEntry {
            vk: keybind.vk,
            modifiers: keybind.mods,
            bypass_trigger: keybind.bypass,
        })
        .collect();
    HookState {
        config: Config {
            trigger_vk: 0x20,
            trigger_mods: 0,
            side: TriggerModifierSide::Any,
            delay_ms: 0,
            timeout_ms: 0,
            double_click: false,
            middle_click: false,
            keybinds,
            match_entries,
        },
        trigger_down: false,
        middle_down: false,
        activated: false,
        timed_out: false,
        capturing: None,
        pressed: HashSet::new(),
        last_release_at: -1,
        version: 0,
        activation_timer_version: None,
        timeout_timer_version: None,
    }
}

#[test]
fn keybind_matching_keeps_trigger_and_bypass_paths_distinct() {
    let mut guard = state_with_keybinds(vec![
        KeybindSnapshot {
            vk: 0x41,
            mods: MOD_CONTROL,
            action: crate::core::actions::WindowAction::LeftHalf,
            cycle: false,
            bypass: false,
        },
        KeybindSnapshot {
            vk: 0x42,
            mods: MOD_ALT,
            action: crate::core::actions::WindowAction::RightHalf,
            cycle: false,
            bypass: true,
        },
    ]);

    assert_eq!(
        matched_keybind_index(&guard, 0x41, MOD_CONTROL, true),
        Some(0)
    );
    assert_eq!(
        matched_keybind_index(&guard, 0x41, MOD_CONTROL, false),
        None
    );
    assert_eq!(matched_keybind_index(&guard, 0x42, MOD_ALT, false), Some(1));
    assert_eq!(matched_keybind_index(&guard, 0x42, MOD_ALT, true), None);

    guard.timed_out = true;
    assert_eq!(
        matched_keybind_index(&guard, 0x41, MOD_CONTROL, false),
        None
    );
}

#[test]
fn capture_decisions_cover_cancel_reject_and_update() {
    assert_eq!(
        capture_decision(Some(&CaptureTarget::Trigger), VK_ESCAPE, 0),
        CaptureDecision::Cancelled
    );
    assert_eq!(
        capture_decision(Some(&CaptureTarget::Trigger), 0x41, MOD_WIN),
        CaptureDecision::Rejected
    );
    assert_eq!(
        capture_decision(
            Some(&CaptureTarget::Keybind("left".to_string())),
            0x41,
            MOD_ALT
        ),
        CaptureDecision::Updated {
            keybind: Some("left".to_string())
        }
    );
}
