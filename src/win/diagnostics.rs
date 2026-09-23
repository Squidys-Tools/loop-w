//! Bounded runtime diagnostics log: user-visible record of failures.
//!
//! Background threads report here instead of failing silently: hook install
//! failures, denied window access, failed frame changes, unavailable monitor
//! data, stale stash records, pipe errors, and settings save failures. The
//! Advanced settings section renders the newest entries; every report also
//! pushes a [`crate::win::events::RuntimeEvent::Diagnostic`] so the UI
//! thread can surface the message in the status line.
//!
//! The store is a ring buffer capped at [`MAX_ENTRIES`] entries. Reporting
//! never blocks: a poisoned or contended lock drops the entry. Consecutive
//! duplicate (kind + message) reports are coalesced so retry loops (pipe
//! reconnect, monitor polling) cannot flood the log.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use super::events::{push, RuntimeEvent};
use crate::core::actions::WindowAction;

/// Maximum retained entries (ring buffer).
pub const MAX_ENTRIES: usize = 100;
/// How many of the newest entries the settings view renders.
pub const VIEW_TAIL: usize = 10;

/// What subsystem produced the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Hook,
    Placement,
    Policy,
    Monitor,
    Stash,
    Ipc,
    Settings,
}

impl DiagnosticKind {
    /// Short stable label, also used on the event bus.
    pub fn label(self) -> &'static str {
        match self {
            DiagnosticKind::Hook => "Hook",
            DiagnosticKind::Placement => "Placement",
            DiagnosticKind::Policy => "Policy",
            DiagnosticKind::Monitor => "Monitor",
            DiagnosticKind::Stash => "Stash",
            DiagnosticKind::Ipc => "IPC",
            DiagnosticKind::Settings => "Settings",
        }
    }

    /// Inverse of [`DiagnosticKind::label`]; unknown labels map to `Ipc`
    /// (the noisiest background source) rather than failing.
    pub fn parse(text: &str) -> Self {
        match text {
            "Hook" => DiagnosticKind::Hook,
            "Placement" => DiagnosticKind::Placement,
            "Policy" => DiagnosticKind::Policy,
            "Monitor" => DiagnosticKind::Monitor,
            "Stash" => DiagnosticKind::Stash,
            "Settings" => DiagnosticKind::Settings,
            _ => DiagnosticKind::Ipc,
        }
    }
}

/// One user-visible diagnostic entry.
#[derive(Debug, Clone)]
pub struct DiagnosticEntry {
    pub seq: u64,
    /// UTC wall-clock `HH:MM:SS` captured at report time.
    pub timestamp: String,
    pub kind: DiagnosticKind,
    /// User-friendly message; never a bare native error code.
    pub message: String,
    /// Optional technical context (API name, action, path). May be empty.
    pub detail: String,
}

impl DiagnosticEntry {
    /// Single-line rendering for the settings view: `[12:03:44] Kind: msg`.
    pub fn headline(&self) -> String {
        format!(
            "[{}] {}: {}",
            self.timestamp,
            self.kind.label(),
            self.message
        )
    }
}

static LOG: OnceLock<Mutex<VecDeque<DiagnosticEntry>>> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(1);

fn log() -> &'static Mutex<VecDeque<DiagnosticEntry>> {
    LOG.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_ENTRIES)))
}

fn timestamp_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    format!(
        "{:02}:{:02}:{:02}",
        (secs / 3600) % 24,
        (secs / 60) % 60,
        secs % 60
    )
}

fn record(kind: DiagnosticKind, message: String, detail: String) -> Option<DiagnosticEntry> {
    let entry = DiagnosticEntry {
        seq: SEQ.fetch_add(1, Ordering::Relaxed),
        timestamp: timestamp_now(),
        kind,
        message,
        detail,
    };
    let mut guard = log().lock().ok()?;
    if let Some(last) = guard.back() {
        if last.kind == entry.kind && last.message == entry.message {
            return None;
        }
    }
    guard.push_back(entry.clone());
    while guard.len() > MAX_ENTRIES {
        guard.pop_front();
    }
    Some(entry)
}

/// Report a diagnostic: append to the bounded log and notify the UI thread.
/// `message` must be user-friendly (no bare native error codes); `detail`
/// carries the technical context for power users.
pub fn report(kind: DiagnosticKind, message: impl Into<String>, detail: impl Into<String>) {
    let Some(entry) = record(kind, message.into(), detail.into()) else {
        return;
    };
    push(RuntimeEvent::Diagnostic {
        kind: entry.kind.label().to_string(),
        message: entry.message.clone(),
        detail: entry.detail.clone(),
    });
}

/// Snapshot of the log, oldest first.
pub fn list() -> Vec<DiagnosticEntry> {
    log()
        .lock()
        .map(|guard| guard.iter().cloned().collect())
        .unwrap_or_default()
}

/// Drop all entries (settings-view Clear button).
pub fn clear() {
    if let Ok(mut guard) = log().lock() {
        guard.clear();
    }
}

/// Current entry count (tests, future badges).
pub fn len() -> usize {
    log().lock().map(|guard| guard.len()).unwrap_or(0)
}

// --- friendly reporters (one per failure source) ---

/// The global keyboard/mouse hook failed to install: hotkeys are dead.
pub fn report_hook_install(detail: &str) {
    report(
        DiagnosticKind::Hook,
        "LoopW couldn't install the global keyboard hook, so hotkeys and the radial trigger won't work. Try restarting LoopW as administrator.",
        detail,
    );
}

/// A window move/resize was rejected or never settled; the window is untouched.
pub fn report_placement(hwnd: u64, detail: &str) {
    report(
        DiagnosticKind::Placement,
        "LoopW couldn't move this window (access may be denied — it may be elevated, protected, or non-resizable; try running LoopW as admin?). The window was left unchanged.",
        format!("hwnd={hwnd}: {detail}"),
    );
}

/// A policy refusal (excluded app, unsupported window); a safe no-op.
pub fn report_policy(diagnostic: &str, action: WindowAction) {
    report(
        DiagnosticKind::Policy,
        format!("{diagnostic} Nothing was changed."),
        format!("action={}", action.display_name()),
    );
}

/// Monitor data was unavailable; layout fell back to the nearest display.
pub fn report_monitor(context: &str) {
    report(
        DiagnosticKind::Monitor,
        "LoopW couldn't read monitor information, so layout falls back to the nearest available display.",
        context,
    );
}

/// Stash pruning dropped records for windows that no longer exist.
pub fn report_stash_stale(removed: usize) {
    report(
        DiagnosticKind::Stash,
        format!(
            "Removed {removed} stale stashed-window record{} for windows that no longer exist.",
            if removed == 1 { "" } else { "s" }
        ),
        "stash prune",
    );
}

/// Named-pipe (IPC) server trouble; the message names the console repro.
pub fn report_ipc(context: &str, detail: &str) {
    report(
        DiagnosticKind::Ipc,
        format!("Pipe command issue: {context}. Reproduce with `LoopW.exe list/actions` from a console while LoopW is running."),
        detail,
    );
}

/// Settings could not be persisted; edits stay in memory only.
pub fn report_settings(message: &str) {
    report(
        DiagnosticKind::Settings,
        message,
        "%LOCALAPPDATA%\\LoopW\\settings.json",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn ring_buffer_caps_at_max_entries() {
        let _guard = lock();
        clear();
        for index in 0..(MAX_ENTRIES + 50) {
            let _ = record(
                DiagnosticKind::Placement,
                format!("move failed #{index}"),
                "test".to_string(),
            );
        }
        assert_eq!(len(), MAX_ENTRIES);
        let entries = list();
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert!(entries.iter().all(|entry| entry.seq > 0));
        // Oldest evicted first: the first retained message is #50.
        assert!(entries.first().unwrap().message.ends_with("#50"));
        clear();
        assert_eq!(len(), 0);
    }

    #[test]
    fn friendly_messages_hide_native_codes() {
        let _guard = lock();
        clear();
        report_hook_install("test");
        report_placement(123, "test");
        report_policy(
            "The target window is non-resizable.",
            WindowAction::LeftHalf,
        );
        report_monitor("test");
        report_stash_stale(2);
        report_ipc("test connect", "test");
        report_settings(
            "LoopW couldn't save settings.json — edits stay in memory until the file is writable again.",
        );
        for entry in list() {
            assert!(
                entry.message.starts_with("LoopW")
                    || entry.message.starts_with("The target")
                    || entry.message.starts_with("Removed")
                    || entry.message.starts_with("Pipe command"),
                "unfriendly: {}",
                entry.message
            );
            assert!(
                !entry.message.contains("0x"),
                "raw code leaked: {}",
                entry.message
            );
        }
        // Every kind roundtrips through its event-bus label.
        for kind in [
            DiagnosticKind::Hook,
            DiagnosticKind::Placement,
            DiagnosticKind::Policy,
            DiagnosticKind::Monitor,
            DiagnosticKind::Stash,
            DiagnosticKind::Ipc,
            DiagnosticKind::Settings,
        ] {
            assert_eq!(DiagnosticKind::parse(kind.label()), kind);
        }
        clear();
    }

    #[test]
    fn report_pushes_store_and_event() {
        let _guard = lock();
        clear();
        super::super::events::drain();
        report(DiagnosticKind::Monitor, "monitor test message", "ctx");
        assert_eq!(len(), 1);
        let stored = list();
        assert_eq!(stored[0].message, "monitor test message");
        let events = super::super::events::drain();
        assert_eq!(events.len(), 1);
        match &events[0] {
            RuntimeEvent::Diagnostic {
                kind,
                message,
                detail,
            } => {
                assert_eq!(kind, "Monitor");
                assert_eq!(message, "monitor test message");
                assert_eq!(detail, "ctx");
                assert_eq!(DiagnosticKind::parse(kind), DiagnosticKind::Monitor);
            }
            other => panic!("expected Diagnostic event, got {other:?}"),
        }
        // Consecutive duplicates coalesce (no log flood, no extra event).
        report(DiagnosticKind::Monitor, "monitor test message", "ctx");
        assert_eq!(len(), 1);
        assert!(super::super::events::drain().is_empty());
        clear();
    }
}
