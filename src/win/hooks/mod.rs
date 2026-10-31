//! Global low-level keyboard/mouse hooks + trigger state machine.
//!
//! Ports `GlobalHotkey`: WH_KEYBOARD_LL always installed, WH_MOUSE_LL
//! installed alongside (middle-click gating is by flag), exact swallow
//! matrix, activation-delay / release-timeout timers guarded by a version
//! counter, double-click gate, bypass/normal keybind matching with repeat
//! suppression, and inline capture mode with Esc cancel + Win rejection.
//!
//! The installing thread pumps a message loop (required for LL delivery).
//! App callbacks are never invoked inline — everything goes through the
//! [`crate::win::events`] queue.

mod capture;
mod consts;
mod state_machine;
#[cfg(test)]
mod tests;
mod thread;

pub use capture::{begin_capture, cancel_capture, is_capturing, CaptureTarget};
pub use thread::{is_active, start, stop};

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use windows::Win32::UI::WindowsAndMessaging::HHOOK;

use crate::core::hotkey::TriggerModifierSide;
use crate::core::keybind_match::MatchEntry;

struct Config {
    trigger_vk: u32,
    trigger_mods: u32,
    side: TriggerModifierSide,
    delay_ms: u64,
    timeout_ms: u64,
    double_click: bool,
    middle_click: bool,
    keybinds: Vec<KeybindSnapshot>,
    match_entries: Vec<MatchEntry>,
}

#[derive(Debug, Clone)]
struct KeybindSnapshot {
    vk: u32,
    mods: u32,
    action: crate::core::actions::WindowAction,
    cycle: bool,
    bypass: bool,
}

struct HookState {
    config: Config,
    trigger_down: bool,
    middle_down: bool,
    activated: bool,
    timed_out: bool,
    capturing: Option<CaptureTarget>,
    pressed: HashSet<u32>,
    last_release_at: i64,
    version: u64,
    activation_timer_version: Option<u64>,
    timeout_timer_version: Option<u64>,
}

struct Handles {
    keyboard: HHOOK,
    mouse: HHOOK,
    thread_id: u32,
    running: bool,
}

// HHOOK/HANDLE values are process-wide opaque handles; sharing them through
// the mutex to install/remove hooks from different threads is safe.
unsafe impl Send for Handles {}

static STATE: OnceLock<Mutex<HookState>> = OnceLock::new();
static HANDLES: OnceLock<Mutex<Handles>> = OnceLock::new();

fn state() -> &'static Mutex<HookState> {
    STATE.get_or_init(|| {
        Mutex::new(HookState {
            config: config_from_shared(),
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
        })
    })
}

fn handles() -> &'static Mutex<Handles> {
    HANDLES.get_or_init(|| {
        Mutex::new(Handles {
            keyboard: HHOOK::default(),
            mouse: HHOOK::default(),
            thread_id: 0,
            running: false,
        })
    })
}

fn config_from_shared() -> Config {
    let settings = super::shared::snapshot();
    let keybinds: Vec<KeybindSnapshot> = settings
        .keybinds
        .iter()
        .map(|keybind| KeybindSnapshot {
            vk: keybind.vk,
            mods: keybind.modifiers,
            action: keybind.action,
            cycle: keybind.cycle_enabled,
            bypass: keybind.bypass_trigger,
        })
        .collect();
    let match_entries = keybinds
        .iter()
        .map(|keybind| MatchEntry {
            vk: keybind.vk,
            modifiers: keybind.mods,
            bypass_trigger: keybind.bypass,
        })
        .collect();
    Config {
        trigger_vk: settings.trigger_vk,
        trigger_mods: settings.trigger_modifiers,
        side: settings.trigger_modifier_side,
        delay_ms: settings.trigger_delay_ms.clamp(0, 1000) as u64,
        timeout_ms: settings.trigger_timeout_ms.clamp(0, 10_000) as u64,
        double_click: settings.double_click_to_trigger,
        middle_click: settings.middle_click_to_trigger,
        keybinds,
        match_entries,
    }
}

/// Refresh config from shared settings; aborts any in-flight press.
/// Use only when trigger binding or behavior changed.
pub fn notify_settings_changed() {
    let config = config_from_shared();
    if let Ok(mut guard) = state().lock() {
        guard.config = config;
        state_machine::reset_input_locked(&mut guard, true);
    }
}

/// Refresh config without disturbing an in-flight press (keybind edits,
/// cosmetic changes). Matches C# SetKeybinds, which deliberately skips
/// the input reset.
pub fn refresh_config() {
    let config = config_from_shared();
    if let Ok(mut guard) = state().lock() {
        guard.config = config;
    }
}
