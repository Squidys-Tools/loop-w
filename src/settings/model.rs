//! `AppSettings` persisted model with C#-compatible JSON keys.
//!
//! JSON keys use the exact C# `AppSettings` property names (`TriggerVk`,
//! `RadialSlots`, ...) so existing files load without loss. Rust field names
//! stay idiomatic; every field carries an explicit `rename`.

use serde::{Deserialize, Serialize};

use crate::core::actions::WindowAction;
use crate::core::hotkey::{MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, TriggerModifierSide, VK_CAPITAL, VK_SPACE};
use crate::core::monitor::MonitorMoveSizePolicy;
use crate::core::radial_targets::{RadialTargetSettings, default_center, default_slots};
use crate::core::stash::StashRecord;

/// A trigger + key combination that applies an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Keybind {
    #[serde(default = "new_id")]
    pub id: String,
    #[serde(default)]
    pub modifiers: u32,
    #[serde(default = "default_space")]
    pub vk: u32,
    #[serde(default = "default_bind_action")]
    pub action: WindowAction,
    #[serde(default = "default_true")]
    pub cycle_enabled: bool,
    #[serde(default, rename = "BypassTrigger")]
    pub bypass_trigger: bool,
}

fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn default_space() -> u32 {
    VK_SPACE
}

fn default_bind_action() -> WindowAction {
    WindowAction::RightHalf
}

fn default_true() -> bool {
    true
}

impl Default for Keybind {
    fn default() -> Self {
        Self {
            id: new_id(),
            modifiers: 0,
            vk: VK_SPACE,
            action: WindowAction::RightHalf,
            cycle_enabled: true,
            bypass_trigger: false,
        }
    }
}

/// Full persisted settings file. Field renames match C# property names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    #[serde(default = "default_trigger_vk", rename = "TriggerVk")]
    pub trigger_vk: u32,
    #[serde(default, rename = "TriggerModifiers")]
    pub trigger_modifiers: u32,
    #[serde(default, rename = "TriggerModifierSide")]
    pub trigger_modifier_side: TriggerModifierSide,
    #[serde(default, rename = "TriggerDelayMilliseconds")]
    pub trigger_delay_ms: i32,
    #[serde(default, rename = "TriggerTimeoutMilliseconds")]
    pub trigger_timeout_ms: i32,
    #[serde(default, rename = "DoubleClickToTrigger")]
    pub double_click_to_trigger: bool,
    #[serde(default, rename = "MiddleClickToTrigger")]
    pub middle_click_to_trigger: bool,
    #[serde(default, rename = "Keybinds")]
    pub keybinds: Vec<Keybind>,
    #[serde(default = "default_slots_serde", rename = "RadialSlots")]
    pub radial_slots: Vec<RadialTargetSettings>,
    #[serde(default = "default_center_serde", rename = "CenterTarget")]
    pub center_target: RadialTargetSettings,
    #[serde(default, rename = "LaunchAtLogin")]
    pub launch_at_login: bool,
    #[serde(default = "default_appearance", rename = "AppearanceMode")]
    pub appearance_mode: String,
    #[serde(default = "default_true", rename = "RadialEnabled")]
    pub radial_enabled: bool,
    #[serde(default = "default_true", rename = "CursorInteractionEnabled")]
    pub cursor_interaction_enabled: bool,
    #[serde(default = "default_outer", rename = "RadialOuterRadius")]
    pub radial_outer_radius: f64,
    #[serde(default = "default_inner", rename = "RadialInnerRadius")]
    pub radial_inner_radius: f64,
    #[serde(default = "default_true", rename = "PreviewEnabled")]
    pub preview_enabled: bool,
    #[serde(default = "default_true", rename = "DragSnapEnabled")]
    pub drag_snap_enabled: bool,
    #[serde(default = "default_snap_threshold", rename = "DragSnapThreshold")]
    pub drag_snap_threshold: i32,
    #[serde(default = "default_true", rename = "RestorePreDragFrameOnSnapCancel")]
    pub restore_pre_drag_on_cancel: bool,
    #[serde(default = "default_true", rename = "StashPersistenceEnabled")]
    pub stash_persistence_enabled: bool,
    #[serde(default, rename = "MonitorMoveSizePolicy")]
    pub monitor_move_policy: MonitorMoveSizePolicy,
    #[serde(default, rename = "GlobalScreenPadding")]
    pub global_padding: i32,
    #[serde(default, rename = "ScreenPaddingLeft")]
    pub padding_left: i32,
    #[serde(default, rename = "ScreenPaddingTop")]
    pub padding_top: i32,
    #[serde(default, rename = "ScreenPaddingRight")]
    pub padding_right: i32,
    #[serde(default, rename = "ScreenPaddingBottom")]
    pub padding_bottom: i32,
    #[serde(default, rename = "ExcludedExecutablePaths")]
    pub excluded_executables: Vec<String>,
    #[serde(default, rename = "ExcludedProcessNames")]
    pub excluded_processes: Vec<String>,
    #[serde(default = "default_peek", rename = "StashEdgePeek")]
    pub stash_peek: i32,
    #[serde(default = "default_hitzone", rename = "StashHitZone")]
    pub stash_hit_zone: i32,
    #[serde(default = "default_reveal_ms", rename = "StashRevealDelayMilliseconds")]
    pub stash_reveal_delay_ms: i32,
    #[serde(default, rename = "StashRecords")]
    pub stash_records: Vec<StashRecord>,
    #[serde(default = "default_preview_padding", rename = "PreviewPadding")]
    pub preview_padding: f64,
    #[serde(default = "default_preview_radius", rename = "PreviewCornerRadius")]
    pub preview_corner_radius: f64,
    #[serde(default = "default_preview_border", rename = "PreviewBorderWidth")]
    pub preview_border_width: f64,
    #[serde(default = "default_accent", rename = "AccentColor")]
    pub accent_color: String,
    #[serde(default = "default_sector_fill", rename = "RadialSectorFill")]
    pub radial_sector_fill: String,
    #[serde(default = "default_sector_stroke", rename = "RadialSectorStroke")]
    pub radial_sector_stroke: String,
    #[serde(default = "default_ring_fill", rename = "RadialRingFill")]
    pub radial_ring_fill: String,
    #[serde(default = "default_preview_border_color", rename = "PreviewBorderColor")]
    pub preview_border_color: String,
}

fn default_trigger_vk() -> u32 {
    VK_CAPITAL
}
fn default_appearance() -> String {
    "Dark".to_string()
}
fn default_outer() -> f64 {
    91.2
}
fn default_inner() -> f64 {
    57.76
}
fn default_snap_threshold() -> i32 {
    24
}
fn default_peek() -> i32 {
    8
}
fn default_hitzone() -> i32 {
    14
}
fn default_reveal_ms() -> i32 {
    80
}
fn default_preview_padding() -> f64 {
    21.0
}
fn default_preview_radius() -> f64 {
    14.0
}
fn default_preview_border() -> f64 {
    2.0
}
fn default_accent() -> String {
    "#3D9BFF".to_string()
}
fn default_sector_fill() -> String {
    "#7A3D9BFF".to_string()
}
fn default_sector_stroke() -> String {
    "#F03D9BFF".to_string()
}
fn default_ring_fill() -> String {
    "#B61B212B".to_string()
}
fn default_preview_border_color() -> String {
    "#B83D9BFF".to_string()
}
fn default_slots_serde() -> Vec<RadialTargetSettings> {
    default_slots()
}
fn default_center_serde() -> RadialTargetSettings {
    default_center()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            trigger_vk: default_trigger_vk(),
            trigger_modifiers: 0,
            trigger_modifier_side: TriggerModifierSide::Any,
            trigger_delay_ms: 0,
            trigger_timeout_ms: 0,
            double_click_to_trigger: false,
            middle_click_to_trigger: false,
            keybinds: Vec::new(),
            radial_slots: default_slots(),
            center_target: default_center(),
            launch_at_login: false,
            appearance_mode: default_appearance(),
            radial_enabled: true,
            cursor_interaction_enabled: true,
            radial_outer_radius: default_outer(),
            radial_inner_radius: default_inner(),
            preview_enabled: true,
            drag_snap_enabled: true,
            drag_snap_threshold: default_snap_threshold(),
            restore_pre_drag_on_cancel: true,
            stash_persistence_enabled: true,
            monitor_move_policy: MonitorMoveSizePolicy::PreservePixels,
            global_padding: 0,
            padding_left: 0,
            padding_top: 0,
            padding_right: 0,
            padding_bottom: 0,
            excluded_executables: Vec::new(),
            excluded_processes: Vec::new(),
            stash_peek: default_peek(),
            stash_hit_zone: default_hitzone(),
            stash_reveal_delay_ms: default_reveal_ms(),
            stash_records: Vec::new(),
            preview_padding: default_preview_padding(),
            preview_corner_radius: default_preview_radius(),
            preview_border_width: default_preview_border(),
            accent_color: default_accent(),
            radial_sector_fill: default_sector_fill(),
            radial_sector_stroke: default_sector_stroke(),
            radial_ring_fill: default_ring_fill(),
            preview_border_color: default_preview_border_color(),
        }
    }
}

impl AppSettings {
    pub fn modifier_mask() -> u32 {
        MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_WIN
    }

    /// Reset every field to factory defaults.
    pub fn reset_all(&mut self) {
        *self = AppSettings::default();
    }

    /// Effective padding edges (global + per-edge).
    pub fn padding_edges(&self) -> (i32, i32, i32, i32) {
        (
            (self.global_padding + self.padding_left).max(0),
            (self.global_padding + self.padding_top).max(0),
            (self.global_padding + self.padding_right).max(0),
            (self.global_padding + self.padding_bottom).max(0),
        )
    }

    /// Whether the effective appearance is light.
    pub fn is_light(&self) -> bool {
        match self.appearance_mode.as_str() {
            "Light" => true,
            "FollowWindows" => is_windows_light_mode(),
            _ => false,
        }
    }
}

#[cfg(windows)]
fn is_windows_light_mode() -> bool {
    // Live registry read lands with the Win32 backend hardening pass.
    // Returning false keeps Dark as the safe default until then.
    false
}

#[cfg(not(windows))]
fn is_windows_light_mode() -> bool {
    false
}
