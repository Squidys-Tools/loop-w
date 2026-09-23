//! Load outcome reporting: clean, migrated/repaired, or which fallback applied.
//!
//! Pure helpers over the persisted JSON text plus the parsed settings. The
//! atomic save/load itself stays in [`super::persistence`]; this module only
//! classifies so the UI can tell "migrated from an older version" apart from
//! "reset to defaults" without changing the C# JSON contract.

use super::model::AppSettings;

/// Why a load fell back to defaults instead of using the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// No file yet (first run).
    Missing,
    /// The file exists but cannot be read (permissions, lock, I/O).
    Unreadable,
    /// Valid prefix cut short: crash or kill mid-write (pre-atomic files).
    Truncated,
    /// Malformed JSON syntax.
    InvalidJson,
    /// Well-formed JSON with wrong types or unknown enum discriminants.
    InvalidTypes,
}

/// How a load resolved: clean, repaired, or defaulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOutcome {
    /// Every key present and `normalize` changed nothing.
    Clean,
    /// An older file: one or more expected keys absent, filled with defaults.
    Migrated,
    /// All keys present but `normalize` clamped or repaired a value.
    Repaired,
    /// The file was unusable; normalized defaults were returned.
    Fallback(FallbackReason),
}

impl LoadOutcome {
    /// Whether the file was ignored in favor of defaults.
    pub fn is_fallback(self) -> bool {
        matches!(self, Self::Fallback(_))
    }

    /// Short human-readable cause for status lines (no native error codes).
    pub fn describe(self) -> &'static str {
        match self {
            Self::Clean => "loaded cleanly",
            Self::Migrated => "migrated from an older version",
            Self::Repaired => "repaired out-of-range values",
            Self::Fallback(FallbackReason::Missing) => "no settings file found",
            Self::Fallback(FallbackReason::Unreadable) => "could not be read",
            Self::Fallback(FallbackReason::Truncated) => "was truncated mid-write",
            Self::Fallback(FallbackReason::InvalidJson) => "contained invalid JSON",
            Self::Fallback(FallbackReason::InvalidTypes) => "contained invalid value types",
        }
    }
}

/// A load plus the reason, so the UI can tell "migrated" from "reset".
#[derive(Debug, Clone)]
pub struct LoadReport {
    pub settings: AppSettings,
    pub outcome: LoadOutcome,
    /// Raw I/O or serde message for logs; UI should prefer `outcome.describe()`.
    pub detail: Option<String>,
}

/// Every persisted top-level PascalCase key. Absence means an older file.
pub const EXPECTED_KEYS: [&str; 41] = [
    "TriggerVk",
    "TriggerModifiers",
    "TriggerModifierSide",
    "TriggerDelayMilliseconds",
    "TriggerTimeoutMilliseconds",
    "DoubleClickToTrigger",
    "MiddleClickToTrigger",
    "Keybinds",
    "RadialSlots",
    "CenterTarget",
    "LaunchAtLogin",
    "AppearanceMode",
    "RadialEnabled",
    "CursorInteractionEnabled",
    "RadialOuterRadius",
    "RadialInnerRadius",
    "PreviewEnabled",
    "DragSnapEnabled",
    "DragSnapThreshold",
    "RestorePreDragFrameOnSnapCancel",
    "StashPersistenceEnabled",
    "MonitorMoveSizePolicy",
    "GlobalScreenPadding",
    "ScreenPaddingLeft",
    "ScreenPaddingTop",
    "ScreenPaddingRight",
    "ScreenPaddingBottom",
    "ExcludedExecutablePaths",
    "ExcludedProcessNames",
    "StashEdgePeek",
    "StashHitZone",
    "StashRevealDelayMilliseconds",
    "StashRecords",
    "PreviewPadding",
    "PreviewCornerRadius",
    "PreviewBorderWidth",
    "AccentColor",
    "RadialSectorFill",
    "RadialSectorStroke",
    "RadialRingFill",
    "PreviewBorderColor",
];

pub fn normalized_defaults() -> AppSettings {
    let mut defaults = AppSettings::default();
    defaults.normalize();
    defaults
}

pub fn fallback(reason: FallbackReason, detail: Option<String>) -> LoadReport {
    LoadReport {
        settings: normalized_defaults(),
        outcome: LoadOutcome::Fallback(reason),
        detail,
    }
}

/// `is_eof` catches truncated/partial writes; `is_data` catches wrong JSON
/// types and out-of-range enum discriminants; the rest is syntax.
pub fn reason_for_parse_error(error: &serde_json::Error) -> FallbackReason {
    if error.is_eof() {
        FallbackReason::Truncated
    } else if error.is_data() {
        FallbackReason::InvalidTypes
    } else {
        FallbackReason::InvalidJson
    }
}

/// Classify a successfully parsed file: missing keys mean an older version
/// (migrated); otherwise a `normalize` diff means repaired values.
pub fn outcome_for(text: &str, parsed: &AppSettings, normalized: &AppSettings) -> LoadOutcome {
    let migrated = match serde_json::from_str::<serde_json::Value>(text) {
        Ok(serde_json::Value::Object(map)) => {
            EXPECTED_KEYS.iter().any(|key| !map.contains_key(*key))
        }
        _ => false,
    };
    if migrated {
        LoadOutcome::Migrated
    } else if normalized != parsed {
        LoadOutcome::Repaired
    } else {
        LoadOutcome::Clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::persistence::{load_detailed_from, save_to};

    fn temp_path() -> (std::path::PathBuf, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("loopw-test-{}", uuid::Uuid::new_v4().simple()));
        let path = dir.join("settings.json");
        std::fs::create_dir_all(&dir).unwrap();
        (dir, path)
    }

    fn expected_defaults() -> AppSettings {
        normalized_defaults()
    }

    #[test]
    fn invalid_types_fall_back_without_partial_apply() {
        let (dir, path) = temp_path();
        std::fs::write(
            &path,
            r#"{"TriggerVk": "CapsLock", "TriggerModifiers": "ctrl"}"#,
        )
        .unwrap();
        let report = load_detailed_from(&path);
        assert_eq!(
            report.outcome,
            LoadOutcome::Fallback(FallbackReason::InvalidTypes)
        );
        assert_eq!(report.settings, expected_defaults());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_syntax_reports_invalid_json() {
        let (dir, path) = temp_path();
        std::fs::write(&path, "{ not json").unwrap();
        let report = load_detailed_from(&path);
        assert_eq!(
            report.outcome,
            LoadOutcome::Fallback(FallbackReason::InvalidJson)
        );
        assert_eq!(report.settings, expected_defaults());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn truncated_partial_write_reports_truncated() {
        let (dir, path) = temp_path();
        // Simulate a kill mid-write: valid prefix cut before the value.
        std::fs::write(&path, r#"{"TriggerVk": 65, "TriggerModifiers":"#).unwrap();
        let report = load_detailed_from(&path);
        assert_eq!(
            report.outcome,
            LoadOutcome::Fallback(FallbackReason::Truncated)
        );
        assert_eq!(report.settings, expected_defaults());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_reports_missing() {
        let (dir, path) = temp_path();
        let missing = dir.join("never-written.json");
        let report = load_detailed_from(&missing);
        assert_eq!(
            report.outcome,
            LoadOutcome::Fallback(FallbackReason::Missing)
        );
        assert_eq!(report.settings, expected_defaults());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let (dir, path) = temp_path();
        let mut saved = AppSettings {
            trigger_vk: 0x41,
            ..AppSettings::default()
        };
        assert!(save_to(&mut saved, &path));
        let text = std::fs::read_to_string(&path).unwrap();
        let with_unknown = text.replacen('{', r#"{"__FutureFeature": 123,"#, 1);
        std::fs::write(&path, with_unknown).unwrap();
        let report = load_detailed_from(&path);
        assert_eq!(report.outcome, LoadOutcome::Clean);
        assert_eq!(report.settings.trigger_vk, 0x41);
        assert_eq!(report.settings, saved);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn out_of_range_numbers_are_clamped_as_repaired() {
        let (dir, path) = temp_path();
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        value["TriggerDelayMilliseconds"] = serde_json::json!(5000);
        value["RadialOuterRadius"] = serde_json::json!(500.0);
        value["DragSnapThreshold"] = serde_json::json!(500);
        value["GlobalScreenPadding"] = serde_json::json!(999);
        std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
        let report = load_detailed_from(&path);
        assert_eq!(report.outcome, LoadOutcome::Repaired);
        assert_eq!(report.settings.trigger_delay_ms, 1000);
        assert_eq!(report.settings.radial_outer_radius, 140.0);
        assert_eq!(report.settings.drag_snap_threshold, 96);
        assert_eq!(report.settings.global_padding, 128);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_version_missing_fields_migrate() {
        let (dir, path) = temp_path();
        // An old file with only the trigger: new fields fill from defaults.
        std::fs::write(&path, r#"{"TriggerVk": 65}"#).unwrap();
        let report = load_detailed_from(&path);
        assert_eq!(report.outcome, LoadOutcome::Migrated);
        assert_eq!(report.settings.trigger_vk, 65);
        assert_eq!(
            report.settings.appearance_mode,
            expected_defaults().appearance_mode
        );
        assert_eq!(report.settings.radial_slots.len(), 8);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
