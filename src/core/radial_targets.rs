//! Persisted radial targets and runtime resolution.
//!
//! A wedge points at nothing, a built-in action, or a stable keybind id.
//! Unknown keybinds normalize to `None` so one bad slot never breaks the file.

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::actions::WindowAction;
use super::cycle::CycleState;
use super::radial::{DEFAULT_ACTIONS, SLOT_COUNT};

/// Persisted target discriminant (matches C# `RadialTargetKind` order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum RadialTargetKind {
    None = 0,
    Action = 1,
    Keybind = 2,
}

/// Persisted wedge/center assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RadialTargetSettings {
    #[serde(default = "default_kind")]
    pub kind: RadialTargetKind,
    #[serde(default = "default_action")]
    pub action: WindowAction,
    #[serde(default)]
    pub keybind_id: String,
    #[serde(default)]
    pub cycle_enabled: bool,
}

fn default_kind() -> RadialTargetKind {
    RadialTargetKind::Action
}

fn default_action() -> WindowAction {
    WindowAction::RightHalf
}

impl Default for RadialTargetSettings {
    fn default() -> Self {
        Self {
            kind: RadialTargetKind::Action,
            action: WindowAction::RightHalf,
            keybind_id: String::new(),
            cycle_enabled: false,
        }
    }
}

impl RadialTargetSettings {
    pub fn none() -> Self {
        Self {
            kind: RadialTargetKind::None,
            action: WindowAction::RightHalf,
            keybind_id: String::new(),
            cycle_enabled: false,
        }
    }

    pub fn action(action: WindowAction) -> Self {
        Self {
            kind: RadialTargetKind::Action,
            action,
            keybind_id: String::new(),
            cycle_enabled: CycleState::can_cycle(action),
        }
    }
}

/// Resolved runtime target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadialTarget {
    None,
    BuiltIn {
        action: WindowAction,
        cycle: bool,
    },
    Keybind {
        id: String,
        action: WindowAction,
        cycle: bool,
        label: String,
    },
}

impl RadialTarget {
    pub fn action(&self) -> Option<WindowAction> {
        match self {
            RadialTarget::None => None,
            RadialTarget::BuiltIn { action, .. } => Some(*action),
            RadialTarget::Keybind { action, .. } => Some(*action),
        }
    }

    pub fn cycle_enabled(&self) -> bool {
        match self {
            RadialTarget::None => false,
            RadialTarget::BuiltIn { cycle, .. } => *cycle,
            RadialTarget::Keybind { cycle, .. } => *cycle,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            RadialTarget::None => "No action".to_string(),
            RadialTarget::BuiltIn { action, .. } => action.display_name().to_string(),
            RadialTarget::Keybind { action, label, .. } => {
                if label.is_empty() {
                    action.display_name().to_string()
                } else {
                    format!("{label} · {}", action.display_name())
                }
            }
        }
    }
}

/// Resolve one persisted slot against the keybind list.
pub fn resolve_slot(slot: &RadialTargetSettings, keybinds: &[ResolvedKeybind]) -> RadialTarget {
    match slot.kind {
        RadialTargetKind::None => RadialTarget::None,
        RadialTargetKind::Action => RadialTarget::BuiltIn {
            action: slot.action,
            cycle: slot.cycle_enabled,
        },
        RadialTargetKind::Keybind => keybinds
            .iter()
            .find(|keybind| keybind.id == slot.keybind_id)
            .map(|keybind| RadialTarget::Keybind {
                id: keybind.id.clone(),
                action: keybind.action,
                cycle: slot.cycle_enabled,
                label: keybind.label.clone(),
            })
            .unwrap_or(RadialTarget::None),
    }
}

/// Minimal keybind view for radial resolution (avoids a settings dep).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedKeybind {
    pub id: String,
    pub action: WindowAction,
    pub label: String,
}

pub fn default_slots() -> Vec<RadialTargetSettings> {
    DEFAULT_ACTIONS
        .iter()
        .map(|a| RadialTargetSettings::action(*a))
        .collect()
}

pub fn default_center() -> RadialTargetSettings {
    RadialTargetSettings::none()
}

/// Normalize a slot list to exactly [`SLOT_COUNT`] safe entries.
pub fn normalize_slots(
    slots: &mut Vec<RadialTargetSettings>,
    keybind_ids: &std::collections::HashSet<String>,
) {
    while slots.len() < SLOT_COUNT {
        slots.push(RadialTargetSettings::none());
    }
    slots.truncate(SLOT_COUNT);
    for slot in slots.iter_mut() {
        normalize_target(slot, keybind_ids);
    }
}

pub fn normalize_target(
    target: &mut RadialTargetSettings,
    keybind_ids: &std::collections::HashSet<String>,
) {
    match target.kind {
        RadialTargetKind::None => {
            target.action = WindowAction::RightHalf;
            target.keybind_id.clear();
            target.cycle_enabled = false;
        }
        RadialTargetKind::Action => {
            if !WindowAction::ALL.contains(&target.action) {
                *target = RadialTargetSettings::none();
            }
        }
        RadialTargetKind::Keybind => {
            if target.keybind_id.is_empty() || !keybind_ids.contains(&target.keybind_id) {
                *target = RadialTargetSettings::none();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn defaults_have_eight_slots_and_empty_center() {
        assert_eq!(default_slots().len(), SLOT_COUNT);
        assert_eq!(default_center().kind, RadialTargetKind::None);
    }

    #[test]
    fn invalid_keybind_target_becomes_none() {
        let mut target = RadialTargetSettings {
            kind: RadialTargetKind::Keybind,
            action: WindowAction::RightHalf,
            keybind_id: "missing".to_string(),
            cycle_enabled: true,
        };
        normalize_target(&mut target, &HashSet::new());
        assert_eq!(target.kind, RadialTargetKind::None);
    }
}
