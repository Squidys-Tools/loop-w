//! Edited-vs-saved commit helper for the settings UI.
//!
//! Strategy is explicit revert-on-failure: the caller snapshots the last
//! saved settings, normalizes the edited candidate, and attempts the disk
//! write *before* touching the shared mirror or notifying services. On
//! success the candidate becomes the new saved value; on failure the UI
//! reverts to the snapshot so the edited control and the saved value stay
//! in sync instead of drifting (unsaved edits silently live in the mirror).
//! The failure message is returned as both `status` (already rendered by
//! every settings section) and `save_error` (sticky until the next success).

use super::model::AppSettings;

/// Shown when Windows or the filesystem rejects a settings save.
pub const SAVE_ERROR_MESSAGE: &str =
    "Could not save settings — your change was reverted. Check file permissions and disk space.";

/// Outcome of one UI commit: what to display, what to share, what to say.
pub struct CommitReport {
    /// Value the UI must display next (candidate on success, revert on fail).
    pub settings: AppSettings,
    pub succeeded: bool,
    pub status: String,
    /// Sticky until the next successful save; `None` on success.
    pub save_error: Option<String>,
    /// True only on success when trigger fields actually changed.
    pub trigger_changed: bool,
}

/// Whether trigger binding/behavior differs (mirrors the C# SetBinding /
// SetTriggerBehavior reset scope: only trigger edits reset in-flight state).
pub fn trigger_changed(before: &AppSettings, after: &AppSettings) -> bool {
    before.trigger_vk != after.trigger_vk
        || before.trigger_modifiers != after.trigger_modifiers
        || before.trigger_modifier_side != after.trigger_modifier_side
        || before.trigger_delay_ms != after.trigger_delay_ms
        || before.trigger_timeout_ms != after.trigger_timeout_ms
        || before.double_click_to_trigger != after.double_click_to_trigger
        || before.middle_click_to_trigger != after.middle_click_to_trigger
}

/// Normalize `candidate`, run `persist`, and revert to `last_saved` on `false`.
///
/// `persist` receives the normalized candidate and returns whether the disk
/// write succeeded (e.g. `|to_save| persistence::save(to_save)`). Services
/// must only be notified when the returned `succeeded` is true.
pub fn commit(
    candidate: AppSettings,
    last_saved: AppSettings,
    persist: impl FnOnce(&mut AppSettings) -> bool,
    success_status: &str,
) -> CommitReport {
    let mut normalized = candidate;
    normalized.normalize();
    let changed = trigger_changed(&last_saved, &normalized);
    let mut to_save = normalized.clone();
    if persist(&mut to_save) {
        CommitReport {
            settings: to_save,
            succeeded: true,
            status: success_status.to_string(),
            save_error: None,
            trigger_changed: changed,
        }
    } else {
        CommitReport {
            settings: last_saved,
            succeeded: false,
            status: SAVE_ERROR_MESSAGE.to_string(),
            save_error: Some(SAVE_ERROR_MESSAGE.to_string()),
            trigger_changed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_keeps_edit_and_clears_error() {
        let saved = AppSettings::default();
        let mut candidate = saved.clone();
        candidate.trigger_delay_ms = 200;
        let report = commit(candidate, saved, |_| true, "Saved");
        assert!(report.succeeded);
        assert_eq!(report.settings.trigger_delay_ms, 200);
        assert_eq!(report.status, "Saved");
        assert_eq!(report.save_error, None);
        assert!(report.trigger_changed);
    }

    #[test]
    fn failure_reverts_edit_and_reports_error() {
        let saved = AppSettings::default();
        let mut candidate = saved.clone();
        candidate.trigger_delay_ms = 200;
        candidate.drag_snap_threshold = 40;
        let report = commit(candidate, saved.clone(), |_| false, "Saved");
        assert!(!report.succeeded);
        // Revert strategy: UI shows the last saved value, not the edit.
        assert_eq!(report.settings, saved);
        assert_eq!(report.status, SAVE_ERROR_MESSAGE);
        assert_eq!(report.save_error.as_deref(), Some(SAVE_ERROR_MESSAGE));
        // Services must not react: nothing actually changed.
        assert!(!report.trigger_changed);
    }

    #[test]
    fn unrelated_edit_reports_no_trigger_change() {
        let saved = AppSettings::default();
        let mut candidate = saved.clone();
        candidate.drag_snap_threshold = 40;
        let report = commit(candidate, saved, |_| true, "Saved");
        assert!(report.succeeded);
        assert!(!report.trigger_changed);
    }
}
