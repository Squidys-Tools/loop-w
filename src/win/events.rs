//! Runtime event bus: background threads → iced UI thread.
//!
//! Hook procs, the pipe server, the display watcher, and tray polling push
//! events here; the UI drains them once per frame (`FrameTick`). Hook
//! callbacks never execute app logic inline, mirroring C# `Dispatch`.

use std::sync::{Mutex, OnceLock};

use crate::core::actions::WindowAction;
use crate::core::rect::{Point, Rect};

static OUTBOX: OnceLock<Mutex<Vec<RuntimeEvent>>> = OnceLock::new();

fn outbox() -> &'static Mutex<Vec<RuntimeEvent>> {
    OUTBOX.get_or_init(|| Mutex::new(Vec::new()))
}

/// Push an event from any thread. Never blocks the hook.
pub fn push(event: RuntimeEvent) {
    if let Ok(mut queue) = outbox().lock() {
        queue.push(event);
    }
}

/// Drain all pending events (UI thread, once per frame).
pub fn drain() -> Vec<RuntimeEvent> {
    outbox()
        .lock()
        .map(|mut queue| core::mem::take(&mut *queue))
        .unwrap_or_default()
}

/// Everything the background runtime can tell the UI.
#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    TriggerPressed,
    TriggerReleased,
    TriggerCancelled,
    TriggerTimedOut,
    KeybindFired {
        action: WindowAction,
        cycle_enabled: bool,
        bypass_trigger: bool,
    },
    RevealStashed,
    CaptureUpdate {
        modifiers: u32,
        vk: u32,
        /// Keybind id when capturing for a keybind, None for the trigger.
        keybind: Option<String>,
    },
    CaptureCancelled,
    CaptureRejected,
    /// A pipe/IPC command arrived and needs execution + reply.
    PipeCommand {
        id: u64,
        command: String,
    },
    /// Second-instance activation signal.
    ShowSettings,
    TrayShowSettings,
    TrayMenuRequested {
        position: Point,
    },
    TrayQuit,
    DisplaysChanged,
    SnapBegin {
        window: u64,
        frame: Rect,
        cursor: Point,
    },
    SnapEnd {
        released: bool,
    },
    /// A background failure worth telling the user about. The entry is
    /// already in the diagnostics log; this event lets the UI surface the
    /// message in the status line. See `win::diagnostics`.
    Diagnostic {
        kind: String,
        message: String,
        detail: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_roundtrips_in_order() {
        drain();
        push(RuntimeEvent::TriggerPressed);
        push(RuntimeEvent::TriggerReleased);
        let events = drain();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], RuntimeEvent::TriggerPressed));
        assert!(drain().is_empty());
    }

    #[test]
    fn diagnostic_roundtrips_with_payload() {
        drain();
        push(RuntimeEvent::Diagnostic {
            kind: "Placement".to_string(),
            message: "LoopW couldn't move this window.".to_string(),
            detail: "test".to_string(),
        });
        let events = drain();
        assert_eq!(events.len(), 1);
        match &events[0] {
            RuntimeEvent::Diagnostic { kind, message, .. } => {
                assert_eq!(kind, "Placement");
                assert!(message.contains("couldn't move"));
            }
            other => panic!("expected Diagnostic, got {other:?}"),
        }
    }
}
