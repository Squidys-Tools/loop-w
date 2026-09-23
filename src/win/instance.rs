//! Single-instance mutex + activation event.
//!
//! Ports `App.TryAcquireInstance`: `Local\LoopW.Instance` mutex decides the
//! first instance; the loser forwards `activate` (or its own command) and
//! exits. The winner watches `Local\LoopW.Activate` for raw wake-ups.

use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::System::Threading::*;

use super::events::{push, RuntimeEvent};

pub const MUTEX_NAME: &str = r"Local\LoopW.Instance";
pub const EVENT_NAME: &str = r"Local\LoopW.Activate";

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(core::iter::once(0)).collect()
}

pub struct InstanceGuard {
    _mutex: HANDLE,
}

// The mutex handle is process-wide and held for the process lifetime;
// moving the guard into the daemon state is safe.
unsafe impl Send for InstanceGuard {}

/// Try to become the resident instance.
/// - `Ok(Some(guard))`: we are first; keep `guard` alive for process life.
/// - `Ok(None)`: another instance owns the mutex.
///
/// `forward` is `Some(command)` when the caller owns pipe delivery of a real
/// CLI command (and needs the reply). In that case this function does **not**
/// substitute a hard-coded `activate` and does not signal the activate event —
/// the caller patient-forwards after seeing `Ok(None)`. `None` means bare
/// activation: pipe `activate` first, then the raw event as fallback.
pub fn acquire(forward: Option<&str>) -> Result<Option<InstanceGuard>, String> {
    let name = wide_null(MUTEX_NAME);
    unsafe {
        match CreateMutexW(None, true, PCWSTR(name.as_ptr())) {
            Ok(mutex) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let _ = CloseHandle(mutex);
                    if forward.is_none() {
                        // Pipe first, raw event only as fallback (C# sends one
                        // activation, not two — a double nudge would focus twice).
                        // Only fall back when activate never hit the wire; a
                        // delivered-but-unanswered pipe call must not re-nudge.
                        if !super::ipc::try_forward_to_running("activate").is_delivered() {
                            signal_event();
                        }
                    }
                    Ok(None)
                } else {
                    start_event_watcher();
                    Ok(Some(InstanceGuard { _mutex: mutex }))
                }
            }
            Err(error) => {
                signal_event();
                Err(format!("Could not acquire the instance mutex: {error}"))
            }
        }
    }
}

/// Signal an existing instance (best effort, no errors).
fn signal_event() {
    let name = wide_null(EVENT_NAME);
    unsafe {
        if let Ok(event) = OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(name.as_ptr())) {
            let _ = SetEvent(event);
            let _ = CloseHandle(event);
        }
    }
}

/// Winner-side watcher: activation signals become ShowSettings events.
fn start_event_watcher() {
    static STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    STARTED.get_or_init(|| {
        std::thread::Builder::new()
            .name("loopw-activate".to_string())
            .spawn(|| {
                let name = wide_null(EVENT_NAME);
                let event = unsafe { CreateEventW(None, false, false, PCWSTR(name.as_ptr())).ok() };
                let Some(event) = event else {
                    return;
                };
                loop {
                    let waited = unsafe { WaitForSingleObject(event, INFINITE) };
                    if waited == WAIT_OBJECT_0 {
                        push(RuntimeEvent::ShowSettings);
                    } else {
                        break;
                    }
                }
            })
            .expect("activate watcher spawns");
    });
}
