//! `WindowAction` catalog: stable discriminants, display names, token parsing.
//!
//! Discriminants must stay in sync with the C# `WindowAction` enum order so
//! existing `settings.json` files (which store actions as integers) keep
//! loading without loss.

use serde_repr::{Deserialize_repr, Serialize_repr};

/// Every window operation LoopW can perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum WindowAction {
    LeftHalf = 0,
    RightHalf = 1,
    TopHalf = 2,
    BottomHalf = 3,
    TopLeftQuarter = 4,
    TopRightQuarter = 5,
    BottomLeftQuarter = 6,
    BottomRightQuarter = 7,
    LeftThird = 8,
    LeftTwoThirds = 9,
    HorizontalCenterThird = 10,
    RightTwoThirds = 11,
    RightThird = 12,
    TopThird = 13,
    TopTwoThirds = 14,
    VerticalCenterThird = 15,
    BottomTwoThirds = 16,
    BottomThird = 17,
    Center = 18,
    Maximize = 19,
    AlmostMaximize = 20,
    Fullscreen = 21,
    MaximizeHeight = 22,
    MaximizeWidth = 23,
    FillAvailableSpace = 24,
    MinimizeOthers = 25,
    HorizontalCenterHalf = 26,
    VerticalCenterHalf = 27,
    FirstFourth = 28,
    SecondFourth = 29,
    ThirdFourth = 30,
    FourthFourth = 31,
    LeftThreeFourths = 32,
    RightThreeFourths = 33,
    NextScreen = 34,
    PreviousScreen = 35,
    LeftScreen = 36,
    RightScreen = 37,
    TopScreen = 38,
    BottomScreen = 39,
    Larger = 40,
    Smaller = 41,
    ScaleUp = 42,
    ScaleDown = 43,
    GrowLeft = 44,
    GrowRight = 45,
    GrowTop = 46,
    GrowBottom = 47,
    GrowHorizontal = 48,
    GrowVertical = 49,
    ShrinkLeft = 50,
    ShrinkRight = 51,
    ShrinkTop = 52,
    ShrinkBottom = 53,
    ShrinkHorizontal = 54,
    ShrinkVertical = 55,
    MoveLeft = 56,
    MoveRight = 57,
    MoveUp = 58,
    MoveDown = 59,
    FocusUp = 60,
    FocusDown = 61,
    FocusLeft = 62,
    FocusRight = 63,
    FocusNextInStack = 64,
    Minimize = 65,
    Stash = 66,
    RevealStashed = 67,
    Hide = 68,
    RestoreInitialFrame = 69,
    Undo = 70,
}

impl WindowAction {
    /// All actions in discriminant order.
    pub const ALL: [WindowAction; 71] = [
        WindowAction::LeftHalf,
        WindowAction::RightHalf,
        WindowAction::TopHalf,
        WindowAction::BottomHalf,
        WindowAction::TopLeftQuarter,
        WindowAction::TopRightQuarter,
        WindowAction::BottomLeftQuarter,
        WindowAction::BottomRightQuarter,
        WindowAction::LeftThird,
        WindowAction::LeftTwoThirds,
        WindowAction::HorizontalCenterThird,
        WindowAction::RightTwoThirds,
        WindowAction::RightThird,
        WindowAction::TopThird,
        WindowAction::TopTwoThirds,
        WindowAction::VerticalCenterThird,
        WindowAction::BottomTwoThirds,
        WindowAction::BottomThird,
        WindowAction::Center,
        WindowAction::Maximize,
        WindowAction::AlmostMaximize,
        WindowAction::Fullscreen,
        WindowAction::MaximizeHeight,
        WindowAction::MaximizeWidth,
        WindowAction::FillAvailableSpace,
        WindowAction::MinimizeOthers,
        WindowAction::HorizontalCenterHalf,
        WindowAction::VerticalCenterHalf,
        WindowAction::FirstFourth,
        WindowAction::SecondFourth,
        WindowAction::ThirdFourth,
        WindowAction::FourthFourth,
        WindowAction::LeftThreeFourths,
        WindowAction::RightThreeFourths,
        WindowAction::NextScreen,
        WindowAction::PreviousScreen,
        WindowAction::LeftScreen,
        WindowAction::RightScreen,
        WindowAction::TopScreen,
        WindowAction::BottomScreen,
        WindowAction::Larger,
        WindowAction::Smaller,
        WindowAction::ScaleUp,
        WindowAction::ScaleDown,
        WindowAction::GrowLeft,
        WindowAction::GrowRight,
        WindowAction::GrowTop,
        WindowAction::GrowBottom,
        WindowAction::GrowHorizontal,
        WindowAction::GrowVertical,
        WindowAction::ShrinkLeft,
        WindowAction::ShrinkRight,
        WindowAction::ShrinkTop,
        WindowAction::ShrinkBottom,
        WindowAction::ShrinkHorizontal,
        WindowAction::ShrinkVertical,
        WindowAction::MoveLeft,
        WindowAction::MoveRight,
        WindowAction::MoveUp,
        WindowAction::MoveDown,
        WindowAction::FocusUp,
        WindowAction::FocusDown,
        WindowAction::FocusLeft,
        WindowAction::FocusRight,
        WindowAction::FocusNextInStack,
        WindowAction::Minimize,
        WindowAction::Stash,
        WindowAction::RevealStashed,
        WindowAction::Hide,
        WindowAction::RestoreInitialFrame,
        WindowAction::Undo,
    ];

    /// Plain-language name shown in settings (never in the radial overlay).
    pub fn display_name(self) -> &'static str {
        match self {
            WindowAction::LeftHalf => "Left half",
            WindowAction::RightHalf => "Right half",
            WindowAction::TopHalf => "Top half",
            WindowAction::BottomHalf => "Bottom half",
            WindowAction::TopLeftQuarter => "Top-left quarter",
            WindowAction::TopRightQuarter => "Top-right quarter",
            WindowAction::BottomLeftQuarter => "Bottom-left quarter",
            WindowAction::BottomRightQuarter => "Bottom-right quarter",
            WindowAction::LeftThird => "Left third",
            WindowAction::LeftTwoThirds => "Left two-thirds",
            WindowAction::HorizontalCenterThird => "Horizontal center third",
            WindowAction::RightTwoThirds => "Right two-thirds",
            WindowAction::RightThird => "Right third",
            WindowAction::TopThird => "Top third",
            WindowAction::TopTwoThirds => "Top two-thirds",
            WindowAction::VerticalCenterThird => "Vertical center third",
            WindowAction::BottomTwoThirds => "Bottom two-thirds",
            WindowAction::BottomThird => "Bottom third",
            WindowAction::Center => "Center",
            WindowAction::Maximize => "Maximize",
            WindowAction::AlmostMaximize => "Almost maximize",
            WindowAction::Fullscreen => "Fullscreen",
            WindowAction::MaximizeHeight => "Maximize height",
            WindowAction::MaximizeWidth => "Maximize width",
            WindowAction::FillAvailableSpace => "Fill available space",
            WindowAction::MinimizeOthers => "Minimize other windows",
            WindowAction::HorizontalCenterHalf => "Horizontal center half",
            WindowAction::VerticalCenterHalf => "Vertical center half",
            WindowAction::FirstFourth => "First fourth",
            WindowAction::SecondFourth => "Second fourth",
            WindowAction::ThirdFourth => "Third fourth",
            WindowAction::FourthFourth => "Fourth fourth",
            WindowAction::LeftThreeFourths => "Left three-fourths",
            WindowAction::RightThreeFourths => "Right three-fourths",
            WindowAction::NextScreen => "Next screen",
            WindowAction::PreviousScreen => "Previous screen",
            WindowAction::LeftScreen => "Screen to the left",
            WindowAction::RightScreen => "Screen to the right",
            WindowAction::TopScreen => "Screen above",
            WindowAction::BottomScreen => "Screen below",
            WindowAction::Larger => "Larger",
            WindowAction::Smaller => "Smaller",
            WindowAction::ScaleUp => "Scale up",
            WindowAction::ScaleDown => "Scale down",
            WindowAction::GrowLeft => "Grow left",
            WindowAction::GrowRight => "Grow right",
            WindowAction::GrowTop => "Grow top",
            WindowAction::GrowBottom => "Grow bottom",
            WindowAction::GrowHorizontal => "Grow horizontally",
            WindowAction::GrowVertical => "Grow vertically",
            WindowAction::ShrinkLeft => "Shrink left",
            WindowAction::ShrinkRight => "Shrink right",
            WindowAction::ShrinkTop => "Shrink top",
            WindowAction::ShrinkBottom => "Shrink bottom",
            WindowAction::ShrinkHorizontal => "Shrink horizontally",
            WindowAction::ShrinkVertical => "Shrink vertically",
            WindowAction::MoveLeft => "Move left",
            WindowAction::MoveRight => "Move right",
            WindowAction::MoveUp => "Move up",
            WindowAction::MoveDown => "Move down",
            WindowAction::FocusUp => "Focus up",
            WindowAction::FocusDown => "Focus down",
            WindowAction::FocusLeft => "Focus left",
            WindowAction::FocusRight => "Focus right",
            WindowAction::FocusNextInStack => "Focus next in stack",
            WindowAction::Minimize => "Minimize",
            WindowAction::Stash => "Stash at edge",
            WindowAction::RevealStashed => "Reveal stashed window",
            WindowAction::Hide => "Hide",
            WindowAction::RestoreInitialFrame => "Restore original frame",
            WindowAction::Undo => "Undo",
        }
    }

    /// CLI token: lowercased variant name without separators.
    pub fn token(self) -> String {
        normalize_token(self.variant_name())
    }

    fn variant_name(self) -> &'static str {
        match self {
            WindowAction::LeftHalf => "LeftHalf",
            WindowAction::RightHalf => "RightHalf",
            WindowAction::TopHalf => "TopHalf",
            WindowAction::BottomHalf => "BottomHalf",
            WindowAction::TopLeftQuarter => "TopLeftQuarter",
            WindowAction::TopRightQuarter => "TopRightQuarter",
            WindowAction::BottomLeftQuarter => "BottomLeftQuarter",
            WindowAction::BottomRightQuarter => "BottomRightQuarter",
            WindowAction::LeftThird => "LeftThird",
            WindowAction::LeftTwoThirds => "LeftTwoThirds",
            WindowAction::HorizontalCenterThird => "HorizontalCenterThird",
            WindowAction::RightTwoThirds => "RightTwoThirds",
            WindowAction::RightThird => "RightThird",
            WindowAction::TopThird => "TopThird",
            WindowAction::TopTwoThirds => "TopTwoThirds",
            WindowAction::VerticalCenterThird => "VerticalCenterThird",
            WindowAction::BottomTwoThirds => "BottomTwoThirds",
            WindowAction::BottomThird => "BottomThird",
            WindowAction::Center => "Center",
            WindowAction::Maximize => "Maximize",
            WindowAction::AlmostMaximize => "AlmostMaximize",
            WindowAction::Fullscreen => "Fullscreen",
            WindowAction::MaximizeHeight => "MaximizeHeight",
            WindowAction::MaximizeWidth => "MaximizeWidth",
            WindowAction::FillAvailableSpace => "FillAvailableSpace",
            WindowAction::MinimizeOthers => "MinimizeOthers",
            WindowAction::HorizontalCenterHalf => "HorizontalCenterHalf",
            WindowAction::VerticalCenterHalf => "VerticalCenterHalf",
            WindowAction::FirstFourth => "FirstFourth",
            WindowAction::SecondFourth => "SecondFourth",
            WindowAction::ThirdFourth => "ThirdFourth",
            WindowAction::FourthFourth => "FourthFourth",
            WindowAction::LeftThreeFourths => "LeftThreeFourths",
            WindowAction::RightThreeFourths => "RightThreeFourths",
            WindowAction::NextScreen => "NextScreen",
            WindowAction::PreviousScreen => "PreviousScreen",
            WindowAction::LeftScreen => "LeftScreen",
            WindowAction::RightScreen => "RightScreen",
            WindowAction::TopScreen => "TopScreen",
            WindowAction::BottomScreen => "BottomScreen",
            WindowAction::Larger => "Larger",
            WindowAction::Smaller => "Smaller",
            WindowAction::ScaleUp => "ScaleUp",
            WindowAction::ScaleDown => "ScaleDown",
            WindowAction::GrowLeft => "GrowLeft",
            WindowAction::GrowRight => "GrowRight",
            WindowAction::GrowTop => "GrowTop",
            WindowAction::GrowBottom => "GrowBottom",
            WindowAction::GrowHorizontal => "GrowHorizontal",
            WindowAction::GrowVertical => "GrowVertical",
            WindowAction::ShrinkLeft => "ShrinkLeft",
            WindowAction::ShrinkRight => "ShrinkRight",
            WindowAction::ShrinkTop => "ShrinkTop",
            WindowAction::ShrinkBottom => "ShrinkBottom",
            WindowAction::ShrinkHorizontal => "ShrinkHorizontal",
            WindowAction::ShrinkVertical => "ShrinkVertical",
            WindowAction::MoveLeft => "MoveLeft",
            WindowAction::MoveRight => "MoveRight",
            WindowAction::MoveUp => "MoveUp",
            WindowAction::MoveDown => "MoveDown",
            WindowAction::FocusUp => "FocusUp",
            WindowAction::FocusDown => "FocusDown",
            WindowAction::FocusLeft => "FocusLeft",
            WindowAction::FocusRight => "FocusRight",
            WindowAction::FocusNextInStack => "FocusNextInStack",
            WindowAction::Minimize => "Minimize",
            WindowAction::Stash => "Stash",
            WindowAction::RevealStashed => "RevealStashed",
            WindowAction::Hide => "Hide",
            WindowAction::RestoreInitialFrame => "RestoreInitialFrame",
            WindowAction::Undo => "Undo",
        }
    }

    /// Parse a CLI token back into an action (case/separator insensitive).
    pub fn parse_token(value: &str) -> Option<WindowAction> {
        Self::ALL
            .iter()
            .copied()
            .find(|a| normalized_eq(value, a.variant_name()))
    }

    /// Numeric discriminant used in persisted JSON.
    pub fn discriminant(self) -> u8 {
        self as u8
    }
}

pub fn normalize_token(value: &str) -> String {
    // ASCII-only lowercase: action names are ASCII, and this avoids the
    // Turkish-I trap of Unicode lowercase (ToLowerInvariant parity).
    value
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Compare action names without allocating a normalized copy for each action.
///
/// Action parsing runs on the command path, where the old implementation
/// allocated once for the input and once for every candidate in `ALL`.
/// Keeping the separator-insensitive comparison streaming preserves the
/// public parsing behavior while making the successful path allocation free.
fn normalized_eq(left: &str, right: &str) -> bool {
    let mut left = left.chars().filter(|c| *c != '-' && *c != '_');
    let mut right = right.chars().filter(|c| *c != '-' && *c != '_');
    loop {
        match (left.next(), right.next()) {
            (Some(left), Some(right)) if left.eq_ignore_ascii_case(&right) => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminants_match_csharp_order() {
        assert_eq!(WindowAction::LeftHalf as u8, 0);
        assert_eq!(WindowAction::RightHalf as u8, 1);
        assert_eq!(WindowAction::Undo as u8, 70);
        assert_eq!(WindowAction::ALL.len(), 71);
    }

    #[test]
    fn every_action_has_a_name_and_roundtrip_token() {
        for action in WindowAction::ALL {
            assert!(!action.display_name().is_empty());
            let token = action.token();
            assert_eq!(WindowAction::parse_token(&token), Some(action));
        }
    }

    #[test]
    fn token_parsing_ignores_case_and_separators() {
        assert_eq!(
            WindowAction::parse_token("left-half"),
            Some(WindowAction::LeftHalf)
        );
        assert_eq!(
            WindowAction::parse_token("LEFTHALF"),
            Some(WindowAction::LeftHalf)
        );
        assert_eq!(
            WindowAction::parse_token("focus_next_in_stack"),
            Some(WindowAction::FocusNextInStack)
        );
    }

    #[test]
    fn token_parsing_rejects_extra_or_non_ascii_text_without_panicking() {
        assert_eq!(WindowAction::parse_token("left-half-extra"), None);
        assert_eq!(WindowAction::parse_token("é-left-half"), None);
    }
}
