//! Window authorization policy (LoopW windows, tool windows, exclusions).
//!
//! Ports `WindowPolicy` decisions that don't need live handles into pure
//! helpers; live checks call back into `crate::core` lists.

/// Minimal decision for tests and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restriction {
    None,
    LoopW,
    Excluded,
    Tool,
    Hidden,
}

/// True when the action needs a resizable target.
pub fn requires_resize(action: crate::core::actions::WindowAction) -> bool {
    use crate::core::actions::WindowAction as A;
    !matches!(
        action,
        A::NextScreen
            | A::PreviousScreen
            | A::LeftScreen
            | A::RightScreen
            | A::TopScreen
            | A::BottomScreen
            | A::MoveLeft
            | A::MoveRight
            | A::MoveUp
            | A::MoveDown
            | A::Center
            | A::Minimize
            | A::Hide
            | A::FocusUp
            | A::FocusDown
            | A::FocusLeft
            | A::FocusRight
            | A::FocusNextInStack
            | A::RestoreInitialFrame
            | A::Undo
    )
}
