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
mod machine;
mod thread;

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use self::machine::reset_input_locked;
use self::thread::hook_thread;
use super::events::{push, RuntimeEvent};
use crate::core::hotkey::TriggerModifierSide;
use crate::core::keybind_match::MatchEntry;

const WH_KEYBOARD_LL: i32 = 13;
const WH_MOUSE_LL: i32 = 14;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;
const WM_MBUTTONDOWN: u32 = 0x0207;
const WM_MBUTTONUP: u32 = 0x0208;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_QUIT_LOOP: u32 = 0x0012;
const WM_TIMER: u32 = 0x0113;
const ACTIVATION_TIMER_ID: usize = 1;
const TIMEOUT_TIMER_ID: usize = 2;

const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
const VK_LWIN: i32 = 0x5B;
const VK_RWIN: i32 = 0x5C;
const VK_ESCAPE: u32 = 0x1B;
const VK_LSHIFT: i32 = 0xA0;
const VK_RSHIFT: i32 = 0xA1;
const VK_LCONTROL: i32 = 0xA2;
const VK_RCONTROL: i32 = 0xA3;
const VK_LMENU: i32 = 0xA4;
const VK_RMENU: i32 = 0xA5;

/// What a capture result applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureTarget {
    Trigger,
    Keybind(String),
}

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

/// Install hooks on a dedicated thread. Returns true when active.
pub fn start() -> bool {
    let Ok(mut guards) = handles().lock() else {
        return false;
    };
    if guards.running {
        return !guards.keyboard.is_invalid();
    }
    guards.running = true;
    drop(guards);
    std::thread::Builder::new()
        .name("loopw-hooks".to_string())
        .spawn(hook_thread)
        .expect("hook thread spawns");
    // Give the thread a moment to install before reporting status.
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(10));
        if let Ok(guards) = handles().lock() {
            if !guards.keyboard.is_invalid() || !guards.running {
                break;
            }
        }
    }
    is_active()
}

pub fn is_active() -> bool {
    handles()
        .lock()
        .map(|guards| !guards.keyboard.is_invalid())
        .unwrap_or(false)
}

/// Remove hooks and stop the pump thread.
pub fn stop() {
    let thread_id = handles().lock().map(|g| g.thread_id).unwrap_or(0);
    if thread_id != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT_LOOP, WPARAM(0), LPARAM(0));
        }
    }
}

/// Refresh config from shared settings; aborts any in-flight press.
/// Use only when trigger binding or behavior changed.
pub fn notify_settings_changed() {
    let config = config_from_shared();
    if let Ok(mut guard) = state().lock() {
        guard.config = config;
        reset_input_locked(&mut guard, true);
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
