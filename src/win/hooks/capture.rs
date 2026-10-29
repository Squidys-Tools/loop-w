//! Inline capture mode for rebinding: the next non-modifier keydown is
//! reported through the events queue; Esc cancels and Win-key combos are
//! rejected.

use super::{
    CaptureTarget, HookState, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};
use crate::core::hotkey::MOD_WIN;
use crate::win::events::{push, RuntimeEvent};

pub(super) fn handle_capture_locked(guard: &mut HookState, vk: u32, modifiers: u32) -> bool {
    if is_modifier_vk(vk) {
        return false;
    }
    let target = guard.capturing.take();
    match capture_decision(target.as_ref(), vk, modifiers) {
        CaptureDecision::Cancelled => push(RuntimeEvent::CaptureCancelled),
        CaptureDecision::Rejected => push(RuntimeEvent::CaptureRejected),
        CaptureDecision::Updated { keybind } => push(RuntimeEvent::CaptureUpdate {
            modifiers,
            vk,
            keybind,
        }),
    }
    true
}

#[derive(Debug, PartialEq, Eq)]
enum CaptureDecision {
    Cancelled,
    Rejected,
    Updated { keybind: Option<String> },
}

fn capture_decision(target: Option<&CaptureTarget>, vk: u32, modifiers: u32) -> CaptureDecision {
    if vk == VK_ESCAPE {
        return CaptureDecision::Cancelled;
    }
    if modifiers & MOD_WIN != 0 {
        return CaptureDecision::Rejected;
    }
    CaptureDecision::Updated {
        keybind: match target {
            Some(CaptureTarget::Keybind(id)) => Some(id.clone()),
            _ => None,
        },
    }
}

fn is_modifier_vk(vk: u32) -> bool {
    matches!(
        vk as i32,
        VK_SHIFT
            | VK_CONTROL
            | VK_MENU
            | VK_LWIN
            | VK_RWIN
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_LMENU
            | VK_RMENU
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::hotkey::MOD_ALT;

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
}
