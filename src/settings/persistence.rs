//! Atomic settings load/save at `%LOCALAPPDATA%\LoopW\settings.json`.
//!
//! Saves go through a temp file + rename so a crash mid-write keeps the last
//! complete file. This answers the compat question: we keep the exact same
//! path and keys as the C# app (zero migration friction) while the loader
//! stays tolerant — unknown fields ignored, missing/invalid fields repaired
//! by [`AppSettings::normalize`]. A brand-new format would orphan every
//! existing user's trigger, keybinds, and radial layout for no runtime gain.

use std::fs;
use std::path::PathBuf;

use super::model::AppSettings;

/// Resolve `%LOCALAPPDATA%\LoopW\settings.json` (or test override).
pub fn settings_path() -> PathBuf {
    if let Ok(override_path) = std::env::var("LOOPW_SETTINGS_PATH") {
        return PathBuf::from(override_path);
    }
    let base = dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("LoopW").join("settings.json")
}

/// Load settings, falling back to normalized defaults on any failure.
pub fn load() -> AppSettings {
    load_from(&settings_path())
}

pub fn load_from(path: &std::path::Path) -> AppSettings {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<AppSettings>(&text) {
            Ok(mut settings) => {
                settings.normalize();
                settings
            }
            Err(_) => {
                let mut defaults = AppSettings::default();
                defaults.normalize();
                defaults
            }
        },
        Err(_) => {
            let mut defaults = AppSettings::default();
            defaults.normalize();
            defaults
        }
    }
}

/// Atomically save settings. Returns `false` instead of corrupting the file.
pub fn save(settings: &mut AppSettings) -> bool {
    save_to(settings, &settings_path())
}

pub fn save_to(settings: &mut AppSettings, path: &std::path::Path) -> bool {
    settings.normalize();
    let serialized = match serde_json::to_string_pretty(settings) {
        Ok(text) => text,
        Err(_) => return false,
    };
    let dir = match path.parent() {
        Some(parent) => parent,
        None => return false,
    };
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let temp_name = format!(
        "{}.{:032x}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("settings.json"),
        uuid::Uuid::new_v4().as_u128()
    );
    let temp_path = dir.join(temp_name);
    if fs::write(&temp_path, format!("{serialized}\n")).is_err() {
        let _ = fs::remove_file(&temp_path);
        return false;
    }
    if fs::rename(&temp_path, path).is_err() {
        let _ = fs::remove_file(&temp_path);
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_values() {
        let dir = std::env::temp_dir().join(format!("loopw-test-{}", uuid::Uuid::new_v4().simple()));
        let path = dir.join("settings.json");
        let mut settings = AppSettings::default();
        settings.trigger_vk = 0x41;
        assert!(save_to(&mut settings, &path));
        let loaded = load_from(&path);
        assert_eq!(loaded.trigger_vk, 0x41);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = std::env::temp_dir().join(format!("loopw-test-{}", uuid::Uuid::new_v4().simple()));
        let path = dir.join("settings.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "{ not json").unwrap();
        let loaded = load_from(&path);
        let mut expected = AppSettings::default();
        expected.normalize();
        assert_eq!(loaded, expected);
        let _ = fs::remove_dir_all(&dir);
    }
}
