//! Nonessential-motion preference: the persisted choice plus the pure
//! decision that combines it with the Windows animation flag.
//!
//! The Windows read itself lives in `crate::win` (`native::animations_enabled`);
//! keeping the resolution here means the behavior is unit-testable without a
//! desktop.

use serde_repr::{Deserialize_repr, Serialize_repr};

/// User preference for nonessential UI motion.
///
/// Discriminants are persisted as integers and stay stable once shipped.
/// `#[serde(other)]` on the default variant repairs an unknown integer in
/// `0..=255` on this field alone; other malformed values (a string, a float,
/// an integer outside `u8`) still fail the whole file, as with every other
/// enum in the settings contract.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum MotionPreference {
    /// Follow the Windows "show animations" setting (the default).
    #[default]
    #[serde(other)]
    FollowWindows = 0,
    /// Minimize nonessential motion even when Windows allows it.
    Reduced = 1,
    /// Allow motion even when Windows asks for less.
    FullMotion = 2,
}

/// All valid preference values (for normalization).
pub const MOTION_PREFERENCES: [MotionPreference; 3] = [
    MotionPreference::FollowWindows,
    MotionPreference::Reduced,
    MotionPreference::FullMotion,
];

impl MotionPreference {
    /// User-facing label for the settings pick list.
    pub fn label(self) -> &'static str {
        match self {
            MotionPreference::FollowWindows => "Follow Windows",
            MotionPreference::Reduced => "Reduced motion",
            MotionPreference::FullMotion => "Full motion",
        }
    }

    /// Preference for a picked label. `None` means the label is not one of
    /// ours, leaving the persisted value untouched.
    pub fn from_label(label: &str) -> Option<Self> {
        MOTION_PREFERENCES
            .into_iter()
            .find(|preference| preference.label() == label)
    }
}

/// Whether nonessential motion should be suppressed.
///
/// `windows_animations_enabled` is the live Windows animation flag. A failed
/// OS read must not claim the user asked for reduced motion, so callers pass
/// `true` when the flag is unknown.
pub fn motion_reduced(preference: MotionPreference, windows_animations_enabled: bool) -> bool {
    match preference {
        MotionPreference::Reduced => true,
        MotionPreference::FullMotion => false,
        MotionPreference::FollowWindows => !windows_animations_enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follow_windows_tracks_the_os_flag() {
        assert!(motion_reduced(MotionPreference::FollowWindows, false));
        assert!(!motion_reduced(MotionPreference::FollowWindows, true));
    }

    #[test]
    fn explicit_choices_override_the_os_flag() {
        assert!(motion_reduced(MotionPreference::Reduced, true));
        assert!(motion_reduced(MotionPreference::Reduced, false));
        assert!(!motion_reduced(MotionPreference::FullMotion, true));
        assert!(!motion_reduced(MotionPreference::FullMotion, false));
    }

    #[test]
    fn every_preference_round_trips_through_repr() {
        for preference in MOTION_PREFERENCES {
            let json = serde_json::to_string(&preference).expect("serialize");
            let back: MotionPreference = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(preference, back);
        }
    }

    #[test]
    fn every_label_round_trips_to_its_preference() {
        for preference in MOTION_PREFERENCES {
            let parsed = MotionPreference::from_label(preference.label());
            assert_eq!(parsed, Some(preference));
        }
        assert_eq!(MotionPreference::from_label("Neon"), None);
    }

    #[test]
    fn unknown_integer_repairs_to_follow_windows() {
        // A hand-edited out-of-range value must not fail the whole file.
        let parsed: MotionPreference = serde_json::from_str("9").expect("catch-all variant");
        assert_eq!(parsed, MotionPreference::FollowWindows);
    }
}
