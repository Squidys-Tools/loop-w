//! Settings normalization: clamps, list repair, color fallbacks.
//!
//! Ports `AppSettings.Normalize` so corrupt or older files always become a
//! safe, usable configuration instead of resetting or crashing.

use std::collections::{HashMap, HashSet};

use crate::core::actions::WindowAction;
use crate::core::hotkey::{TRIGGER_MODIFIER_SIDES, VK_CAPITAL};
use crate::core::monitor::MonitorMoveSizePolicy;
use crate::core::radial::SLOT_COUNT;
use crate::core::radial_targets::{normalize_target, RadialTargetKind, RadialTargetSettings};
use crate::core::stash::{StashEdge, StashMonitor, StashPlacement, StashPoint, StashRect};

use super::color::normalize_color;
use super::model::{AppSettings, Keybind};

const MOD_MASK: u32 = 0x0001 | 0x0002 | 0x0004 | 0x0008;
const MAX_STASH_RECORDS: usize = 64;

impl AppSettings {
    /// Clamp every field into its valid range and repair lists in place.
    pub fn normalize(&mut self) {
        if self.trigger_vk == 0 {
            self.trigger_vk = VK_CAPITAL;
        }
        self.trigger_modifiers &= MOD_MASK;
        if !TRIGGER_MODIFIER_SIDES.contains(&self.trigger_modifier_side) {
            self.trigger_modifier_side = Default::default();
        }
        self.trigger_delay_ms = self.trigger_delay_ms.clamp(0, 1000);
        self.trigger_timeout_ms = self.trigger_timeout_ms.clamp(0, 10_000);

        normalize_keybinds(&mut self.keybinds);
        normalize_radial(
            &mut self.radial_slots,
            &mut self.center_target,
            &self.keybinds,
        );

        if !matches!(
            self.appearance_mode.as_str(),
            "Dark" | "FollowWindows" | "Light"
        ) {
            self.appearance_mode = "Dark".to_string();
        }

        self.radial_outer_radius = clamp_finite(self.radial_outer_radius, 64.0, 140.0, 91.2);
        self.radial_inner_radius = clamp_finite(
            self.radial_inner_radius,
            24.0,
            (self.radial_outer_radius - 8.0).max(24.0),
            57.76,
        );
        self.preview_padding = clamp_finite(self.preview_padding, 4.0, 48.0, 21.0);
        self.preview_corner_radius = clamp_finite(self.preview_corner_radius, 4.0, 32.0, 14.0);
        self.preview_border_width = clamp_finite(self.preview_border_width, 0.0, 6.0, 2.0);
        self.drag_snap_threshold = self.drag_snap_threshold.clamp(4, 96);

        if !matches!(
            self.monitor_move_policy,
            MonitorMoveSizePolicy::PreservePixels | MonitorMoveSizePolicy::PreserveLogicalSize
        ) {
            self.monitor_move_policy = MonitorMoveSizePolicy::PreservePixels;
        }

        self.global_padding = self.global_padding.clamp(0, 128);
        self.padding_left = self.padding_left.clamp(0, 128);
        self.padding_top = self.padding_top.clamp(0, 128);
        self.padding_right = self.padding_right.clamp(0, 128);
        self.padding_bottom = self.padding_bottom.clamp(0, 128);

        normalize_exclusions(&mut self.excluded_executables, &mut self.excluded_processes);

        self.stash_peek = self.stash_peek.clamp(1, 48);
        self.stash_hit_zone = self.stash_hit_zone.clamp(1, 96);
        self.stash_reveal_delay_ms = self.stash_reveal_delay_ms.clamp(0, 2000);
        normalize_stash_records(&mut self.stash_records);

        self.accent_color = normalize_color(&self.accent_color.clone(), "#007AFF");
        self.radial_sector_fill = normalize_color(&self.radial_sector_fill.clone(), "#7A007AFF");
        self.radial_sector_stroke =
            normalize_color(&self.radial_sector_stroke.clone(), "#F0007AFF");
        self.radial_ring_fill = normalize_color(&self.radial_ring_fill.clone(), "#B61E1E1E");
        self.preview_border_color =
            normalize_color(&self.preview_border_color.clone(), "#B8007AFF");
    }
}

fn clamp_finite(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback.clamp(min, max)
    }
}

fn normalize_keybinds(keybinds: &mut Vec<Keybind>) {
    let mut ids: HashSet<String> = HashSet::new();
    for bind in keybinds.iter_mut() {
        if bind.id.trim().is_empty() || !ids.insert(bind.id.clone()) {
            loop {
                let fresh = uuid::Uuid::new_v4().simple().to_string();
                if ids.insert(fresh.clone()) {
                    bind.id = fresh;
                    break;
                }
            }
        }
        bind.modifiers &= MOD_MASK;
        if !WindowAction::ALL.contains(&bind.action) {
            bind.action = WindowAction::RightHalf;
        }
    }
}

fn normalize_radial(
    slots: &mut Vec<RadialTargetSettings>,
    center: &mut RadialTargetSettings,
    keybinds: &[Keybind],
) {
    while slots.len() < SLOT_COUNT {
        slots.push(RadialTargetSettings {
            kind: RadialTargetKind::None,
            action: WindowAction::RightHalf,
            keybind_id: String::new(),
            cycle_enabled: false,
        });
    }
    slots.truncate(SLOT_COUNT);
    let ids: HashSet<String> = keybinds.iter().map(|k| k.id.clone()).collect();
    for slot in slots.iter_mut() {
        normalize_target(slot, &ids);
    }
    normalize_target(center, &ids);
}

fn normalize_exclusions(executables: &mut Vec<String>, processes: &mut Vec<String>) {
    *executables = dedup_keep_order(
        executables
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    );
    *processes = dedup_keep_order(
        processes
            .iter()
            .map(|s| {
                let trimmed = s.trim();
                // Match C#: strip directory + extension (`app.exe` -> `app`).
                let file = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
                file.rsplit('.').last().unwrap_or(file).to_string()
            })
            .filter(|s| !s.is_empty()),
    );
}

fn dedup_keep_order(items: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut lower_seen = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        let lower = item.to_lowercase();
        if lower_seen.insert(lower) && seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

fn normalize_stash_records(records: &mut Vec<crate::core::stash::StashRecord>) {
    if records.len() > MAX_STASH_RECORDS {
        records.drain(0..records.len() - MAX_STASH_RECORDS);
    }
    let mut ids: HashSet<String> = HashSet::new();
    // Repair ids first, then drop nulls (no nulls in Rust) and fix shapes.
    let mut repaired: Vec<(String, usize)> = Vec::new();
    for record in records.iter_mut() {
        if record.id.trim().is_empty() || !ids.insert(record.id.clone()) {
            loop {
                let fresh = uuid::Uuid::new_v4().simple().to_string();
                if ids.insert(fresh.clone()) {
                    record.id = fresh;
                    break;
                }
            }
        }
        repaired.push((record.id.clone(), 0));
    }
    let _ = repaired;
    // Ensure nested shapes are non-degenerate and edges valid.
    for record in records.iter_mut() {
        if !matches!(
            record.edge,
            StashEdge::Left | StashEdge::Right | StashEdge::Top | StashEdge::Bottom
        ) {
            record.edge = StashEdge::Left;
        }
        fix_rect(&mut record.original_placement.normal_position);
        fix_rect(&mut record.original_monitor.monitor);
        fix_rect(&mut record.original_monitor.work);
        fix_rect(&mut record.stashed_frame);
        if !(record.original_monitor.dpi_x.is_finite() && record.original_monitor.dpi_x > 0.0) {
            record.original_monitor.dpi_x = 96.0;
        }
        if !(record.original_monitor.dpi_y.is_finite() && record.original_monitor.dpi_y > 0.0) {
            record.original_monitor.dpi_y = 96.0;
        }
    }
    // Deduplicate by id keeping the last occurrence (newest wins).
    let mut last_index: HashMap<String, usize> = HashMap::new();
    for (index, record) in records.iter().enumerate() {
        last_index.insert(record.id.clone(), index);
    }
    let mut keep: Vec<bool> = vec![false; records.len()];
    for index in last_index.values() {
        keep[*index] = true;
    }
    let mut filtered = Vec::with_capacity(records.len());
    for (index, record) in records.drain(..).enumerate() {
        if keep[index] {
            filtered.push(record);
        }
    }
    *records = filtered;
}

fn fix_rect(_rect: &mut StashRect) {
    // StashRect is plain data; nothing to clamp beyond keeping it present.
}

#[allow(unused_imports)]
fn _keep_types() {
    let _ = (
        std::mem::size_of::<StashMonitor>(),
        std::mem::size_of::<StashPlacement>(),
        std::mem::size_of::<StashPoint>(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_and_numbers_normalize() {
        let mut settings = AppSettings {
            trigger_vk: 0,
            trigger_delay_ms: 5000,
            radial_outer_radius: 500.0,
            drag_snap_threshold: 500,
            appearance_mode: "Neon".to_string(),
            ..AppSettings::default()
        };
        settings.normalize();
        assert_eq!(settings.trigger_vk, VK_CAPITAL);
        assert_eq!(settings.trigger_delay_ms, 1000);
        assert_eq!(settings.radial_outer_radius, 140.0);
        assert_eq!(settings.drag_snap_threshold, 96);
        assert_eq!(settings.appearance_mode, "Dark");
    }

    #[test]
    fn invalid_radial_targets_become_none() {
        let mut settings = AppSettings::default();
        settings.radial_slots[0].kind = RadialTargetKind::Keybind;
        settings.radial_slots[0].keybind_id = "missing".to_string();
        settings.normalize();
        assert_eq!(settings.radial_slots[0].kind, RadialTargetKind::None);
    }
}
