//! Process-wide shared state: settings snapshot + service singletons.
//!
//! The iced UI thread owns persistence; background threads (hooks, pipe
//! server, display watcher) read through the shared settings handle so
//! behavior changes apply without restart and no hook can stay logically
//! stuck on stale config.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};

use crate::settings::AppSettings;

static SHARED_SETTINGS: OnceLock<std::sync::Arc<RwLock<AppSettings>>> = OnceLock::new();
static SETTINGS_REVISION: AtomicU64 = AtomicU64::new(1);

/// Install the shared settings handle. Call once at startup.
pub fn init(settings: AppSettings) -> std::sync::Arc<RwLock<AppSettings>> {
    let shared = std::sync::Arc::new(RwLock::new(settings));
    let _ = SHARED_SETTINGS.set(shared.clone());
    shared
}

/// Read a snapshot of the current settings (cloned; cheap enough at hook rate).
pub fn snapshot() -> AppSettings {
    SHARED_SETTINGS
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Read the one live setting used by the radial cursor hot path without
/// cloning the complete settings model.
pub fn cursor_interaction_enabled() -> bool {
    SHARED_SETTINGS
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|guard| guard.cursor_interaction_enabled)
        .unwrap_or(true)
}

/// Clone only the live stash records for the settings mirror. Runtime ticks
/// should not clone the complete settings model just to detect stash changes.
pub fn stash_records() -> Vec<crate::core::stash::StashRecord> {
    SHARED_SETTINGS
        .get()
        .and_then(|lock| lock.read().ok())
        .map(|guard| guard.stash_records.clone())
        .unwrap_or_default()
}

/// Monotonic revision for updates made through the shared settings mirror.
///
/// The UI uses this to avoid cloning the stash list on every resident timer
/// tick. A revision may advance for an unrelated settings edit; that is still
/// cheap to compare and only causes a stash clone after an actual update.
pub fn settings_revision() -> u64 {
    SETTINGS_REVISION.load(Ordering::Relaxed)
}

/// Mutate the shared settings in place.
pub fn update(change: impl FnOnce(&mut AppSettings)) {
    if let Some(lock) = SHARED_SETTINGS.get() {
        if let Ok(mut guard) = lock.write() {
            change(&mut guard);
            SETTINGS_REVISION.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Replace the whole shared settings value (after load/reset).
pub fn replace(settings: AppSettings) {
    update(|slot| *slot = settings);
}

/// Normalize + atomically persist the shared settings. Safe from any thread.
pub fn save_now() -> bool {
    let Some(lock) = SHARED_SETTINGS.get() else {
        return false;
    };
    let Ok(mut guard) = lock.write() else {
        return false;
    };
    crate::settings::persistence::save(&mut guard)
}
