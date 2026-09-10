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

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::events::{push, RuntimeEvent};
use crate::core::hotkey::{TriggerModifierSide, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN};
use crate::core::keybind_match::{match_keybind, MatchEntry};

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
    Config {
        trigger_vk: settings.trigger_vk,
        trigger_mods: settings.trigger_modifiers,
        side: settings.trigger_modifier_side,
        delay_ms: settings.trigger_delay_ms.clamp(0, 1000) as u64,
        timeout_ms: settings.trigger_timeout_ms.clamp(0, 10_000) as u64,
        double_click: settings.double_click_to_trigger,
        middle_click: settings.middle_click_to_trigger,
        keybinds: settings
            .keybinds
            .iter()
            .map(|keybind| KeybindSnapshot {
                vk: keybind.vk,
                mods: keybind.modifiers,
                action: keybind.action,
                cycle: keybind.cycle_enabled,
                bypass: keybind.bypass_trigger,
            })
            .collect(),
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

fn hook_thread() {
    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let handle = handles().lock().map(|g| g.keyboard).unwrap_or_default();
        if code < 0 {
            return CallNextHookEx(Some(handle), code, wparam, lparam);
        }
        let message = wparam.0 as u32;
        let is_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
        let is_up = message == WM_KEYUP || message == WM_SYSKEYUP;
        if !is_down && !is_up {
            return CallNextHookEx(Some(handle), code, wparam, lparam);
        }
        let vk = (*(lparam.0 as *const KBDLLHOOKSTRUCT)).vkCode;
        let swallow = state()
            .lock()
            .map(|mut guard| {
                if is_down {
                    handle_key_down_locked(&mut guard, vk)
                } else {
                    handle_key_up_locked(&mut guard, vk)
                }
            })
            .unwrap_or(false);
        if swallow {
            LRESULT(1)
        } else {
            CallNextHookEx(Some(handle), code, wparam, lparam)
        }
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let handle = handles().lock().map(|g| g.mouse).unwrap_or_default();
        if code < 0 {
            return CallNextHookEx(Some(handle), code, wparam, lparam);
        }
        let message = wparam.0 as u32;
        let swallow = state().lock().map(|mut guard| match message {
            WM_MBUTTONDOWN => handle_middle_down_locked(&mut guard),
            WM_MBUTTONUP => handle_middle_up_locked(&mut guard),
            _ => false,
        });
        // Title-bar drag traffic goes to the snap tracker (never swallowed);
        // middle-button traffic drives the alternate trigger only.
        match message {
            WM_LBUTTONDOWN => super::snap_service::note_button(true),
            WM_LBUTTONUP => super::snap_service::note_button(false),
            _ => {}
        }
        if swallow.unwrap_or(false) {
            LRESULT(1)
        } else {
            CallNextHookEx(Some(handle), code, wparam, lparam)
        }
    }

    unsafe {
        let keyboard = SetWindowsHookExW(
            WINDOWS_HOOK_ID(WH_KEYBOARD_LL),
            Some(keyboard_proc),
            None,
            0,
        )
        .unwrap_or_default();
        let mouse = SetWindowsHookExW(WINDOWS_HOOK_ID(WH_MOUSE_LL), Some(mouse_proc), None, 0)
            .unwrap_or_default();
        if let Ok(mut guards) = handles().lock() {
            guards.keyboard = keyboard;
            guards.mouse = mouse;
            guards.thread_id = GetCurrentThreadId();
        }
        if keyboard.is_invalid() {
            super::diagnostics::report_hook_install("SetWindowsHookExW failed for WH_KEYBOARD_LL");
        } else if mouse.is_invalid() {
            super::diagnostics::report_hook_install(
                "SetWindowsHookExW failed for WH_MOUSE_LL; middle-click trigger unavailable",
            );
        }
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if let Ok(guards) = handles().lock() {
            if !guards.keyboard.is_invalid() {
                let _ = UnhookWindowsHookEx(guards.keyboard);
            }
            if !guards.mouse.is_invalid() {
                let _ = UnhookWindowsHookEx(guards.mouse);
            }
        }
        if let Ok(mut guards) = handles().lock() {
            guards.keyboard = HHOOK::default();
            guards.mouse = HHOOK::default();
            guards.thread_id = 0;
            guards.running = false;
        }
        if let Ok(mut guard) = state().lock() {
            reset_input_locked(&mut guard, false);
        }
    }
}

// --- state machine (all callers hold the lock) ---

fn any_held(guard: &HookState) -> bool {
    guard.trigger_down || guard.middle_down
}

fn handle_key_down_locked(guard: &mut HookState, vk: u32) -> bool {
    if guard.capturing.is_some() {
        return handle_capture_locked(guard, vk);
    }
    if vk == guard.config.trigger_vk
        && current_modifiers(guard.config.side) == guard.config.trigger_mods
    {
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
    let entries: Vec<MatchEntry> = guard
        .config
        .keybinds
        .iter()
        .map(|keybind| MatchEntry {
            vk: keybind.vk,
            modifiers: keybind.mods,
            bypass_trigger: keybind.bypass,
        })
        .collect();
    let matched = if held && !guard.timed_out {
        match_keybind(
            &entries,
            vk,
            current_modifiers(TriggerModifierSide::Any),
            guard.config.trigger_vk,
            true,
        )
    } else if !held {
        match_keybind(
            &entries,
            vk,
            current_modifiers(TriggerModifierSide::Any),
            guard.config.trigger_vk,
            false,
        )
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

fn handle_key_up_locked(guard: &mut HookState, vk: u32) -> bool {
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

fn handle_middle_down_locked(guard: &mut HookState) -> bool {
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

fn handle_middle_up_locked(guard: &mut HookState) -> bool {
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
    let now = super::native::tick_count() as i64;
    let second = guard.last_release_at >= 0
        && now - guard.last_release_at <= super::native::double_click_time() as i64;
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
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(delay));
        if let Ok(mut guard) = state().lock() {
            if guard.version == version && any_held(&guard) {
                activate_locked(&mut guard, version);
            }
        }
    });
}

fn activate_locked(guard: &mut HookState, version: u64) {
    if version != guard.version || !any_held(guard) || guard.activated || guard.timed_out {
        return;
    }
    guard.activated = true;
    push(RuntimeEvent::TriggerPressed);
    if guard.config.timeout_ms > 0 {
        let timeout = guard.config.timeout_ms;
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(timeout));
            if let Ok(mut guard) = state().lock() {
                if guard.version == version && any_held(&guard) && guard.activated {
                    guard.activated = false;
                    guard.timed_out = true;
                    guard.version += 1;
                    guard.pressed.clear();
                    push(RuntimeEvent::TriggerTimedOut);
                }
            }
        });
    }
}

fn complete_release_locked(guard: &mut HookState) {
    guard.version += 1;
    let was_activated = guard.activated;
    guard.activated = false;
    guard.timed_out = false;
    guard.pressed.clear();
    guard.last_release_at = if guard.config.double_click {
        super::native::tick_count() as i64
    } else {
        -1
    };
    if was_activated {
        push(RuntimeEvent::TriggerReleased);
    }
}

fn reset_input_locked(guard: &mut HookState, notify: bool) {
    let was_active = guard.activated;
    guard.version += 1;
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

fn handle_capture_locked(guard: &mut HookState, vk: u32) -> bool {
    if is_modifier_vk(vk) {
        return false;
    }
    let target = guard.capturing.take();
    if vk == VK_ESCAPE {
        push(RuntimeEvent::CaptureCancelled);
        return true;
    }
    let mods = current_modifiers(TriggerModifierSide::Any);
    if mods & MOD_WIN != 0 {
        push(RuntimeEvent::CaptureRejected);
        return true;
    }
    push(RuntimeEvent::CaptureUpdate {
        modifiers: mods,
        vk,
        keybind: match target {
            Some(CaptureTarget::Keybind(id)) => Some(id),
            _ => None,
        },
    });
    true
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

fn current_modifiers(side: TriggerModifierSide) -> u32 {
    let mut mods = 0u32;
    let (shift, ctrl, alt) = match side {
        TriggerModifierSide::Left => (VK_LSHIFT, VK_LCONTROL, VK_LMENU),
        TriggerModifierSide::Right => (VK_RSHIFT, VK_RCONTROL, VK_RMENU),
        TriggerModifierSide::Any => (VK_SHIFT, VK_CONTROL, VK_MENU),
    };
    if super::native::async_key_down(shift) {
        mods |= MOD_SHIFT;
    }
    if super::native::async_key_down(ctrl) {
        mods |= MOD_CONTROL;
    }
    if super::native::async_key_down(alt) {
        mods |= MOD_ALT;
    }
    let win = match side {
        TriggerModifierSide::Left => super::native::async_key_down(VK_LWIN),
        TriggerModifierSide::Right => super::native::async_key_down(VK_RWIN),
        TriggerModifierSide::Any => {
            super::native::async_key_down(VK_LWIN) || super::native::async_key_down(VK_RWIN)
        }
    };
    if win {
        mods |= MOD_WIN;
    }
    mods
}
