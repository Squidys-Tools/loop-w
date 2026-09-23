//! Directional cycle chains (half -> third -> two-thirds).
//!
//! Mirrors `WindowCycleService`: repeating the same requested action advances
//! through its chain. The cursor only advances after a successful placement,
//! so callers must invoke [`CycleState::commit`] on success.

use std::collections::HashMap;

use super::actions::WindowAction;

/// One cycle step selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleSelection {
    pub requested: WindowAction,
    pub effective: WindowAction,
    pub position: usize,
    pub count: usize,
    pub is_cycle: bool,
}

impl CycleSelection {
    pub fn status_suffix(self) -> String {
        if self.is_cycle {
            format!("  ·  Cycle {}/{}", self.position + 1, self.count)
        } else {
            String::new()
        }
    }
}

/// Per-window cycle cursors.
#[derive(Debug, Default)]
pub struct CycleState {
    cursors: HashMap<u64, (WindowAction, usize)>,
}

impl CycleState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn can_cycle(action: WindowAction) -> bool {
        chain_for(action).is_some()
    }

    pub fn select(
        &mut self,
        window: u64,
        requested: WindowAction,
        enabled: bool,
    ) -> CycleSelection {
        let Some(chain) = chain_for(requested) else {
            self.cursors.remove(&window);
            return CycleSelection {
                requested,
                effective: requested,
                position: 0,
                count: 0,
                is_cycle: false,
            };
        };
        if !enabled {
            self.cursors.remove(&window);
            return CycleSelection {
                requested,
                effective: requested,
                position: 0,
                count: 0,
                is_cycle: false,
            };
        }
        let requested_pos = chain.iter().position(|a| *a == requested).unwrap_or(0);
        let mut position = requested_pos;
        if let Some((prev_requested, prev_pos)) = self.cursors.get(&window) {
            if *prev_requested == requested {
                position = (prev_pos + 1) % chain.len();
            }
        }
        CycleSelection {
            requested,
            effective: chain[position],
            position,
            count: chain.len(),
            is_cycle: true,
        }
    }

    pub fn commit(&mut self, window: u64, selection: CycleSelection) {
        if selection.is_cycle {
            self.cursors
                .insert(window, (selection.requested, selection.position));
        }
    }

    pub fn forget(&mut self, window: u64) {
        self.cursors.remove(&window);
    }
}

fn chain_for(action: WindowAction) -> Option<&'static [WindowAction; 3]> {
    use WindowAction::*;
    match action {
        LeftHalf | LeftThird | LeftTwoThirds => Some(&[LeftHalf, LeftThird, LeftTwoThirds]),
        RightHalf | RightThird | RightTwoThirds => Some(&[RightHalf, RightThird, RightTwoThirds]),
        TopHalf | TopThird | TopTwoThirds => Some(&[TopHalf, TopThird, TopTwoThirds]),
        BottomHalf | BottomThird | BottomTwoThirds => {
            Some(&[BottomHalf, BottomThird, BottomTwoThirds])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_cyclable_actions_pass_through() {
        let mut state = CycleState::new();
        let sel = state.select(1, WindowAction::Center, true);
        assert!(!sel.is_cycle);
        assert_eq!(sel.effective, WindowAction::Center);
    }

    #[test]
    fn cardinal_actions_cycle_three_steps() {
        let mut state = CycleState::new();
        let first = state.select(7, WindowAction::RightHalf, true);
        assert_eq!(first.effective, WindowAction::RightHalf);
        state.commit(7, first);
        let second = state.select(7, WindowAction::RightHalf, true);
        assert_eq!(second.effective, WindowAction::RightThird);
        state.commit(7, second);
        let third = state.select(7, WindowAction::RightHalf, true);
        assert_eq!(third.effective, WindowAction::RightTwoThirds);
    }

    #[test]
    fn disabled_cycle_clears_cursor() {
        let mut state = CycleState::new();
        let first = state.select(9, WindowAction::LeftHalf, true);
        state.commit(9, first);
        let sel = state.select(9, WindowAction::LeftHalf, false);
        assert!(!sel.is_cycle);
    }
}
