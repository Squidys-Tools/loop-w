//! Capture mode: the settings UI grabs the next non-modifier key as a
//! binding. While armed, capture takes precedence over trigger/keybind
//! matching; Esc cancels, Win combos are rejected, anything else updates
//! the target.

use super::consts::{
    VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL,
    VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};
use super::state_machine::reset_input_locked;
use super::{state, HookState};
use crate::core::hotkey::MOD_WIN;
use crate::win::events::{push, RuntimeEvent};

/// What a capture result applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureTarget {
    Trigger,
    Keybind(String),
}

pub fn begin_capture(target: CaptureTarget) {
    if let Ok(mut guard) = state().lock() {
        guard.capturing = Some(target);
        reset_input_locked(&mut guard, true);
    }
}

pub fn cancel_capture() {
    if let Ok(mut guard) = state().lock() {
        if guard.capturing.take().is_some() {
            push(RuntimeEvent::CaptureCancelled);
        }
    }
}

pub fn is_capturing() -> bool {
    state()
        .lock()
        .map(|g| g.capturing.is_some())
        .unwrap_or(false)
}

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
pub(super) enum CaptureDecision {
    Cancelled,
    Rejected,
    Updated { keybind: Option<String> },
}

pub(super) fn capture_decision(
    target: Option<&CaptureTarget>,
    vk: u32,
    modifiers: u32,
) -> CaptureDecision {
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
