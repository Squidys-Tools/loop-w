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
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("settings.json"),
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
    use crate::core::actions::WindowAction;
    use crate::core::hotkey::TriggerModifierSide;
    use crate::core::monitor::MonitorMoveSizePolicy;
    use crate::core::radial_targets::RadialTargetKind;
    use crate::core::stash::StashEdge;

    #[test]
    fn roundtrip_preserves_values() {
        let dir =
            std::env::temp_dir().join(format!("loopw-test-{}", uuid::Uuid::new_v4().simple()));
        let path = dir.join("settings.json");
        let mut settings = AppSettings::default();
        settings.trigger_vk = 0x41;
        assert!(save_to(&mut settings, &path));
        let loaded = load_from(&path);
        assert_eq!(loaded.trigger_vk, 0x41);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A `settings.json` exactly as the C# build writes it (indented JSON,
    /// PascalCase keys, integer enums, `N`-format GUIDs) must load with zero
    /// loss: every value below is already in normalized form, so the loader
    /// must preserve it bit-for-bit.
    #[test]
    fn csharp_settings_file_loads_without_loss() {
        let dir =
            std::env::temp_dir().join(format!("loopw-test-{}", uuid::Uuid::new_v4().simple()));
        let path = dir.join("settings.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, CSHARP_FIXTURE).unwrap();
        let loaded = load_from(&path);

        assert_eq!(loaded.trigger_vk, 0x41);
        assert_eq!(loaded.trigger_modifiers, 0x6);
        assert_eq!(loaded.trigger_modifier_side, TriggerModifierSide::Left);
        assert_eq!(loaded.trigger_delay_ms, 150);
        assert_eq!(loaded.trigger_timeout_ms, 3000);
        assert!(loaded.double_click_to_trigger);
        assert!(loaded.middle_click_to_trigger);

        assert_eq!(loaded.keybinds.len(), 2);
        assert_eq!(loaded.keybinds[0].id, "a1b2c3d4e5f60718293a4b5c6d7e8f90");
        assert_eq!(loaded.keybinds[0].modifiers, 0x6);
        assert_eq!(loaded.keybinds[0].vk, 0x42);
        assert_eq!(loaded.keybinds[0].action, WindowAction::NextScreen);
        assert!(loaded.keybinds[0].cycle_enabled);
        assert!(!loaded.keybinds[0].bypass_trigger);
        assert_eq!(loaded.keybinds[1].id, "00112233445566778899aabbccddeeff");
        assert_eq!(loaded.keybinds[1].modifiers, 0x2);
        assert_eq!(loaded.keybinds[1].vk, 0x4D);
        assert_eq!(loaded.keybinds[1].action, WindowAction::Minimize);
        assert!(!loaded.keybinds[1].cycle_enabled);
        assert!(loaded.keybinds[1].bypass_trigger);

        assert_eq!(loaded.radial_slots.len(), 8);
        assert_eq!(loaded.radial_slots[0].kind, RadialTargetKind::Action);
        assert_eq!(loaded.radial_slots[0].action, WindowAction::RightHalf);
        assert!(loaded.radial_slots[0].cycle_enabled);
        assert_eq!(
            loaded.radial_slots[1].action,
            WindowAction::BottomRightQuarter
        );
        assert!(!loaded.radial_slots[1].cycle_enabled);
        assert_eq!(loaded.radial_slots[2].kind, RadialTargetKind::Keybind);
        assert_eq!(
            loaded.radial_slots[2].keybind_id,
            "00112233445566778899aabbccddeeff"
        );
        assert!(loaded.radial_slots[2].cycle_enabled);
        assert_eq!(loaded.radial_slots[3].action, WindowAction::LeftHalf);
        assert_eq!(loaded.radial_slots[4].action, WindowAction::TopLeftQuarter);
        assert_eq!(loaded.radial_slots[5].kind, RadialTargetKind::None);
        assert_eq!(loaded.radial_slots[5].keybind_id, "");
        assert!(!loaded.radial_slots[5].cycle_enabled);
        assert_eq!(loaded.radial_slots[6].action, WindowAction::TopHalf);
        assert_eq!(loaded.radial_slots[7].action, WindowAction::TopRightQuarter);
        assert_eq!(loaded.center_target.kind, RadialTargetKind::None);

        assert!(loaded.launch_at_login);
        assert_eq!(loaded.appearance_mode, "Light");
        assert!(!loaded.radial_enabled);
        assert!(loaded.cursor_interaction_enabled);
        assert_eq!(loaded.radial_outer_radius, 100.0);
        assert_eq!(loaded.radial_inner_radius, 60.0);
        assert!(loaded.preview_enabled);
        assert!(loaded.drag_snap_enabled);
        assert_eq!(loaded.drag_snap_threshold, 32);
        assert!(!loaded.restore_pre_drag_on_cancel);
        assert!(loaded.stash_persistence_enabled);
        assert_eq!(
            loaded.monitor_move_policy,
            MonitorMoveSizePolicy::PreserveLogicalSize
        );
        assert_eq!(loaded.global_padding, 8);
        assert_eq!(loaded.padding_left, 4);
        assert_eq!(loaded.padding_top, 6);
        assert_eq!(loaded.padding_right, 8);
        assert_eq!(loaded.padding_bottom, 10);
        assert_eq!(
            loaded.excluded_executables,
            vec!["C:\\Tools\\obs64\\obs64.exe".to_string()]
        );
        assert_eq!(
            loaded.excluded_processes,
            vec!["obs64".to_string(), "game".to_string()]
        );
        assert_eq!(loaded.stash_peek, 6);
        assert_eq!(loaded.stash_hit_zone, 20);
        assert_eq!(loaded.stash_reveal_delay_ms, 120);

        assert_eq!(loaded.stash_records.len(), 1);
        let record = &loaded.stash_records[0];
        assert_eq!(record.id, "deadbeefdeadbeefdeadbeefdeadbeef");
        assert_eq!(record.executable_path, "C:\\Windows\\System32\\notepad.exe");
        assert_eq!(record.process_id, 1234);
        assert_eq!(record.window_class, "Notepad");
        assert_eq!(record.title, "Untitled - Notepad");
        assert_eq!(record.edge, StashEdge::Right);
        assert_eq!(record.original_placement.length, 44);
        assert_eq!(record.original_placement.flags, 0);
        assert_eq!(record.original_placement.show_command, 1);
        assert_eq!(record.original_placement.min_position.x, 0);
        assert_eq!(record.original_placement.min_position.y, 0);
        assert_eq!(record.original_placement.max_position.x, -1);
        assert_eq!(record.original_placement.max_position.y, -1);
        assert_eq!(record.original_placement.normal_position.left, 100);
        assert_eq!(record.original_placement.normal_position.top, 100);
        assert_eq!(record.original_placement.normal_position.right, 900);
        assert_eq!(record.original_placement.normal_position.bottom, 700);
        assert_eq!(record.original_monitor.monitor.right, 1920);
        assert_eq!(record.original_monitor.monitor.bottom, 1080);
        assert_eq!(record.original_monitor.work.bottom, 1040);
        assert_eq!(record.original_monitor.dpi_x, 144.0);
        assert_eq!(record.original_monitor.dpi_y, 144.0);
        assert_eq!(record.stashed_frame.left, 1912);
        assert_eq!(record.stashed_frame.right, 1920);

        assert_eq!(loaded.preview_padding, 24.0);
        assert_eq!(loaded.preview_corner_radius, 16.0);
        assert_eq!(loaded.preview_border_width, 3.0);
        assert_eq!(loaded.accent_color, "#FF007AFF");
        assert_eq!(loaded.radial_sector_fill, "#7A007AFF");
        assert_eq!(loaded.radial_sector_stroke, "#F0007AFF");
        assert_eq!(loaded.radial_ring_fill, "#B61E1E1E");
        assert_eq!(loaded.preview_border_color, "#B8007AFF");

        // Re-saving must keep the same keys and values (no silent migration).
        let mut resaved = loaded.clone();
        assert!(save_to(&mut resaved, &path));
        let reloaded = load_from(&path);
        assert_eq!(reloaded, resaved);

        let _ = fs::remove_dir_all(&dir);
    }

    /// Byte-for-byte shape of a C# `JsonSerializer.Serialize(AppSettings)`
    /// file with non-default values in every section.
    const CSHARP_FIXTURE: &str = r##"{
  "TriggerVk": 65,
  "TriggerModifiers": 6,
  "TriggerModifierSide": 1,
  "TriggerDelayMilliseconds": 150,
  "TriggerTimeoutMilliseconds": 3000,
  "DoubleClickToTrigger": true,
  "MiddleClickToTrigger": true,
  "Keybinds": [
    {
      "Id": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
      "Modifiers": 6,
      "Vk": 66,
      "Action": 34,
      "CycleEnabled": true,
      "BypassTrigger": false
    },
    {
      "Id": "00112233445566778899aabbccddeeff",
      "Modifiers": 2,
      "Vk": 77,
      "Action": 65,
      "CycleEnabled": false,
      "BypassTrigger": true
    }
  ],
  "RadialSlots": [
    { "Kind": 1, "Action": 1, "KeybindId": "", "CycleEnabled": true },
    { "Kind": 1, "Action": 7, "KeybindId": "", "CycleEnabled": false },
    { "Kind": 2, "Action": 19, "KeybindId": "00112233445566778899aabbccddeeff", "CycleEnabled": true },
    { "Kind": 1, "Action": 0, "KeybindId": "", "CycleEnabled": false },
    { "Kind": 1, "Action": 4, "KeybindId": "", "CycleEnabled": false },
    { "Kind": 0, "Action": 1, "KeybindId": "", "CycleEnabled": false },
    { "Kind": 1, "Action": 2, "KeybindId": "", "CycleEnabled": true },
    { "Kind": 1, "Action": 5, "KeybindId": "", "CycleEnabled": false }
  ],
  "CenterTarget": { "Kind": 0, "Action": 1, "KeybindId": "", "CycleEnabled": false },
  "LaunchAtLogin": true,
  "AppearanceMode": "Light",
  "RadialEnabled": false,
  "CursorInteractionEnabled": true,
  "RadialOuterRadius": 100,
  "RadialInnerRadius": 60,
  "PreviewEnabled": true,
  "DragSnapEnabled": true,
  "DragSnapThreshold": 32,
  "RestorePreDragFrameOnSnapCancel": false,
  "StashPersistenceEnabled": true,
  "MonitorMoveSizePolicy": 1,
  "GlobalScreenPadding": 8,
  "ScreenPaddingLeft": 4,
  "ScreenPaddingTop": 6,
  "ScreenPaddingRight": 8,
  "ScreenPaddingBottom": 10,
  "ExcludedExecutablePaths": ["C:\\Tools\\obs64\\obs64.exe"],
  "ExcludedProcessNames": ["obs64", "game"],
  "StashEdgePeek": 6,
  "StashHitZone": 20,
  "StashRevealDelayMilliseconds": 120,
  "StashRecords": [
    {
      "Id": "deadbeefdeadbeefdeadbeefdeadbeef",
      "ExecutablePath": "C:\\Windows\\System32\\notepad.exe",
      "ProcessId": 1234,
      "WindowClass": "Notepad",
      "Title": "Untitled - Notepad",
      "Edge": 1,
      "OriginalPlacement": {
        "Length": 44,
        "Flags": 0,
        "ShowCommand": 1,
        "MinPosition": { "X": 0, "Y": 0 },
        "MaxPosition": { "X": -1, "Y": -1 },
        "NormalPosition": { "Left": 100, "Top": 100, "Right": 900, "Bottom": 700 }
      },
      "OriginalMonitor": {
        "Monitor": { "Left": 0, "Top": 0, "Right": 1920, "Bottom": 1080 },
        "Work": { "Left": 0, "Top": 0, "Right": 1920, "Bottom": 1040 },
        "DpiX": 144,
        "DpiY": 144
      },
      "StashedFrame": { "Left": 1912, "Top": 100, "Right": 1920, "Bottom": 700 }
    }
  ],
  "PreviewPadding": 24,
  "PreviewCornerRadius": 16,
  "PreviewBorderWidth": 3,
  "AccentColor": "#FF007AFF",
  "RadialSectorFill": "#7A007AFF",
  "RadialSectorStroke": "#F0007AFF",
  "RadialRingFill": "#B61E1E1E",
  "PreviewBorderColor": "#B8007AFF"
}"##;

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir =
            std::env::temp_dir().join(format!("loopw-test-{}", uuid::Uuid::new_v4().simple()));
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
