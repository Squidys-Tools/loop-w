//! Trigger state machine: every reducer runs with the hook-state lock
//! already held by the caller (the LL hook procs, the pump's `WM_TIMER`
//! arm, and the config/capture entry points). Reducers decide
//! swallow-vs-pass and queue `RuntimeEvent`s — no app logic runs inline.

use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

use super::capture::handle_capture_locked;
use super::consts::{
    ACTIVATION_TIMER_ID, TIMEOUT_TIMER_ID, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};
use super::{state, HookState};
use crate::core::hotkey::{TriggerModifierSide, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};
use crate::core::keybind_match::match_keybind;
use crate::win::events::{push, RuntimeEvent};
use crate::win::native;

fn any_held(guard: &HookState) -> bool {
    guard.trigger_down || guard.middle_down
}

pub(super) fn handle_key_down_locked(guard: &mut HookState, vk: u32) -> bool {
    let trigger_modifiers = current_modifiers(guard.config.side);
    let keybind_modifiers = if guard.config.side == TriggerModifierSide::Any {
        trigger_modifiers
    } else {
        current_modifiers(TriggerModifierSide::Any)
    };
    handle_key_down_with_modifiers_locked(guard, vk, trigger_modifiers, keybind_modifiers)
}

/// Testable key-down reducer. The hook callback supplies the two modifier
/// snapshots because trigger-side matching and keybind matching deliberately
/// use different side policies.
fn handle_key_down_with_modifiers_locked(
    guard: &mut HookState,
    vk: u32,
    trigger_modifiers: u32,
    keybind_modifiers: u32,
) -> bool {
    if guard.capturing.is_some() {
        return handle_capture_locked(guard, vk, keybind_modifiers);
    }
    if vk == guard.config.trigger_vk && trigger_modifiers == guard.config.trigger_mods {
        if !guard.trigger_down {
            guard.trigger_down = true;
            if !guard.middle_down {
                begin_press_locked(guard);
            }
        }
        return true;
    }
    let held = any_held(guard);
    if guard.pressed.contains(&vk) {
        // Auto-repeat: swallow, fire nothing.
        return true;
    }
    let matched = if held && !guard.timed_out {
        matched_keybind_index(guard, vk, keybind_modifiers, true)
    } else if !held {
        matched_keybind_index(guard, vk, keybind_modifiers, false)
    } else {
        None
    };
    if let Some(index) = matched {
        let snapshot = guard.config.keybinds[index].clone();
        guard.pressed.insert(vk);
        if snapshot.action == crate::core::actions::WindowAction::RevealStashed {
            push(RuntimeEvent::RevealStashed);
        } else {
            push(RuntimeEvent::KeybindFired {
                action: snapshot.action,
                cycle_enabled: snapshot.cycle,
                bypass_trigger: snapshot.bypass,
            });
        }
        return true;
    }
    false
}

/// Match a keybind without reading Win32 modifier state or emitting events.
/// This keeps the hook's policy decisions deterministic and directly
/// testable while the callback remains responsible only for side effects.
pub(super) fn matched_keybind_index(
    guard: &HookState,
    vk: u32,
    modifiers: u32,
    trigger_held: bool,
) -> Option<usize> {
    match_keybind(
        &guard.config.match_entries,
        vk,
        modifiers,
        guard.config.trigger_vk,
        trigger_held,
    )
}

pub(super) fn handle_key_up_locked(guard: &mut HookState, vk: u32) -> bool {
    guard.pressed.remove(&vk);
    if guard.capturing.is_some() {
        return false;
    }
    if vk != guard.config.trigger_vk || !guard.trigger_down {
        return false;
    }
    guard.trigger_down = false;
    if !any_held(guard) {
        complete_release_locked(guard);
    }
    true
}

pub(super) fn handle_middle_down_locked(guard: &mut HookState) -> bool {
    if guard.capturing.is_some() || !guard.config.middle_click {
        return false;
    }
    if !guard.middle_down {
        guard.middle_down = true;
        if !guard.trigger_down {
            begin_press_locked(guard);
        }
    }
    true
}

pub(super) fn handle_middle_up_locked(guard: &mut HookState) -> bool {
    if !guard.middle_down {
        return false;
    }
    guard.middle_down = false;
    if !any_held(guard) {
        complete_release_locked(guard);
    }
    true
}

fn begin_press_locked(guard: &mut HookState) {
    if !guard.config.double_click {
        schedule_activation_locked(guard);
        return;
    }
    let now = native::tick_count() as i64;
    let second = guard.last_release_at >= 0
        && now - guard.last_release_at <= native::double_click_time() as i64;
    guard.last_release_at = -1;
    if second {
        schedule_activation_locked(guard);
    }
}

fn schedule_activation_locked(guard: &mut HookState) {
    guard.version += 1;
    let version = guard.version;
    if guard.config.delay_ms == 0 {
        activate_locked(guard, version);
        return;
    }
    let delay = guard.config.delay_ms;
    guard.activation_timer_version = Some(version);
    unsafe {
        let _ = SetTimer(
            None,
            ACTIVATION_TIMER_ID,
            delay.min(u32::MAX as u64) as u32,
            None,
        );
    }
}

fn activate_locked(guard: &mut HookState, version: u64) {
    if version != guard.version || !any_held(guard) || guard.activated || guard.timed_out {
        return;
    }
    guard.activation_timer_version = None;
    unsafe {
        let _ = KillTimer(None, ACTIVATION_TIMER_ID);
    }
    guard.activated = true;
    push(RuntimeEvent::TriggerPressed);
    if guard.config.timeout_ms > 0 {
        let timeout = guard.config.timeout_ms;
        guard.timeout_timer_version = Some(version);
        unsafe {
            let _ = SetTimer(
                None,
                TIMEOUT_TIMER_ID,
                timeout.min(u32::MAX as u64) as u32,
                None,
            );
        }
    }
}

pub(super) fn handle_timer(timer_id: usize) {
    let Ok(mut guard) = state().lock() else {
        return;
    };
    let version = match timer_id {
        ACTIVATION_TIMER_ID => guard.activation_timer_version.take(),
        TIMEOUT_TIMER_ID => guard.timeout_timer_version.take(),
        _ => None,
    };
    unsafe {
        let _ = KillTimer(None, timer_id);
    }
    let Some(version) = version else {
        return;
    };
    if timer_id == ACTIVATION_TIMER_ID {
        if guard.version == version && any_held(&guard) {
            activate_locked(&mut guard, version);
        }
    } else if timer_id == TIMEOUT_TIMER_ID
        && guard.version == version
        && any_held(&guard)
        && guard.activated
    {
        guard.activated = false;
        guard.timed_out = true;
        guard.version += 1;
        guard.pressed.clear();
        push(RuntimeEvent::TriggerTimedOut);
    }
}

fn complete_release_locked(guard: &mut HookState) {
    guard.version += 1;
    guard.activation_timer_version = None;
    guard.timeout_timer_version = None;
    unsafe {
        let _ = KillTimer(None, ACTIVATION_TIMER_ID);
        let _ = KillTimer(None, TIMEOUT_TIMER_ID);
    }
    let was_activated = guard.activated;
    guard.activated = false;
    guard.timed_out = false;
    guard.pressed.clear();
    guard.last_release_at = if guard.config.double_click {
        native::tick_count() as i64
    } else {
        -1
    };
    if was_activated {
        push(RuntimeEvent::TriggerReleased);
    }
}

pub(super) fn reset_input_locked(guard: &mut HookState, notify: bool) {
    let was_active = guard.activated;
    guard.version += 1;
    guard.activation_timer_version = None;
    guard.timeout_timer_version = None;
    unsafe {
        let _ = KillTimer(None, ACTIVATION_TIMER_ID);
        let _ = KillTimer(None, TIMEOUT_TIMER_ID);
    }
    guard.trigger_down = false;
    guard.middle_down = false;
    guard.activated = false;
    guard.timed_out = false;
    guard.last_release_at = -1;
    guard.pressed.clear();
    if notify && was_active {
        push(RuntimeEvent::TriggerCancelled);
    }
}

fn current_modifiers(side: TriggerModifierSide) -> u32 {
    let mut mods = 0u32;
    let (shift, ctrl, alt) = match side {
        TriggerModifierSide::Left => (VK_LSHIFT, VK_LCONTROL, VK_LMENU),
        TriggerModifierSide::Right => (VK_RSHIFT, VK_RCONTROL, VK_RMENU),
        TriggerModifierSide::Any => (VK_SHIFT, VK_CONTROL, VK_MENU),
    };
    if native::async_key_down(shift) {
        mods |= MOD_SHIFT;
    }
    if native::async_key_down(ctrl) {
        mods |= MOD_CONTROL;
    }
    if native::async_key_down(alt) {
        mods |= MOD_ALT;
    }
    let win = match side {
        TriggerModifierSide::Left => native::async_key_down(VK_LWIN),
        TriggerModifierSide::Right => native::async_key_down(VK_RWIN),
        TriggerModifierSide::Any => {
            native::async_key_down(VK_LWIN) || native::async_key_down(VK_RWIN)
        }
    };
    if win {
        mods |= MOD_WIN;
    }
    mods
}
