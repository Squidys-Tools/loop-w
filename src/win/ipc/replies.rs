//! Pending pipe replies, completed on the UI thread.

use std::sync::{Condvar, OnceLock};
use std::time::Duration;

use super::wire::READ_TIMEOUT_MS;

/// Pending pipe replies, completed on the UI thread.
static REPLIES: OnceLock<std::sync::Mutex<std::collections::HashMap<u64, String>>> =
    OnceLock::new();
static REPLY_WAKE: OnceLock<Condvar> = OnceLock::new();

fn replies() -> &'static std::sync::Mutex<std::collections::HashMap<u64, String>> {
    REPLIES.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn reply_wake() -> &'static Condvar {
    REPLY_WAKE.get_or_init(Condvar::new)
}

pub fn take_reply(id: u64) -> Option<String> {
    replies().lock().ok()?.remove(&id)
}

/// Store the UI thread's reply for a pending pipe command.
pub fn set_reply(id: u64, reply: String) {
    if let Ok(mut slots) = replies().lock() {
        slots.insert(id, reply);
        reply_wake().notify_one();
    }
}

pub(super) fn wait_reply(id: u64) -> Option<String> {
    let deadline = std::time::Instant::now() + Duration::from_millis(READ_TIMEOUT_MS);
    let mut slots = replies().lock().ok()?;
    loop {
        if let Some(reply) = slots.remove(&id) {
            return Some(reply);
        }
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return slots.remove(&id);
        }
        let (next, timeout) = reply_wake().wait_timeout(slots, remaining).ok()?;
        slots = next;
        if timeout.timed_out() {
            return slots.remove(&id);
        }
    }
}
