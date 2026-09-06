//! Process-wide shared state: settings snapshot + service singletons.
//!
//! The iced UI thread owns persistence; background threads (hooks, pipe
//! server, display watcher) read through the shared settings handle so
//! behavior changes apply without restart and no hook can stay logically
//! stuck on stale config.

use std::sync::{OnceLock, RwLock};

use crate::settings::AppSettings;

static SHARED_SETTINGS: OnceLock<std::sync::Arc<RwLock<AppSettings>>> = OnceLock::new();

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

/// Mutate the shared settings in place.
pub fn update(change: impl FnOnce(&mut AppSettings)) {
    if let Some(lock) = SHARED_SETTINGS.get() {
        if let Ok(mut guard) = lock.write() {
            change(&mut guard);
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
