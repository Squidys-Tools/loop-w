//! Focus navigation math: nearest window by direction, next in z-order.
//!
//! Ports `WindowNavigation.TryFindDirectional` / `TryFindNextInStack`.
//! Perpendicular distance dominates; primary distance breaks ties.

use super::rect::Rect;

/// Cardinal focus direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavDir {
    Left,
    Right,
    Up,
    Down,
}

/// Nearest candidate strictly in `dir` of `source` (centers compared).
/// Candidates are `(id, frame)` in z-order; returns the winning id.
pub fn find_directional(source: Rect, candidates: &[(u64, Rect)], dir: NavDir) -> Option<u64> {
    let (scx, scy) = center(source);
    let mut best: Option<(u64, i32, i32)> = None; // (id, perp, primary)
    for (id, frame) in candidates {
        let (ccx, ccy) = center(*frame);
        if !in_direction((scx, scy), (ccx, ccy), dir) {
            continue;
        }
        let (perp, primary) = match dir {
            NavDir::Left | NavDir::Right => (
                interval_gap(source.top, source.bottom, frame.top, frame.bottom),
                (scx - ccx).abs(),
            ),
            NavDir::Up | NavDir::Down => (
                interval_gap(source.left, source.right, frame.left, frame.right),
                (scy - ccy).abs(),
            ),
        };
        let take = match best {
            None => true,
            Some((_, bq, bp)) => perp < bq || (perp == bq && primary < bp),
        };
        if take {
            best = Some((*id, perp, primary));
        }
    }
    best.map(|(id, _, _)| id)
}

/// Next window in z-order after `current`, wrapping around.
pub fn find_next_in_stack(candidates: &[u64], current: u64) -> Option<u64> {
    if candidates.is_empty() {
        return None;
    }
    let index = candidates.iter().position(|id| *id == current);
    let start = match index {
        None => 0,
        Some(i) => (i + 1) % candidates.len(),
    };
    for offset in 0..candidates.len() {
        let id = candidates[(start + offset) % candidates.len()];
        if id != current {
            return Some(id);
        }
    }
    None
}

fn center(rect: Rect) -> (i32, i32) {
    (rect.left + rect.width() / 2, rect.top + rect.height() / 2)
}

fn in_direction(source: (i32, i32), candidate: (i32, i32), dir: NavDir) -> bool {
    match dir {
        NavDir::Left => candidate.0 < source.0,
        NavDir::Right => candidate.0 > source.0,
        NavDir::Up => candidate.1 < source.1,
        NavDir::Down => candidate.1 > source.1,
    }
}

/// Zero when projections overlap, else the edge gap.
fn interval_gap(a0: i32, a1: i32, b0: i32, b1: i32) -> i32 {
    if a1 >= b0 && b1 >= a0 {
        0
    } else if b0 > a1 {
        b0 - a1
    } else {
        a0 - b1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chooses_nearest_window_to_the_right() {
        let current = Rect::new(400, 300, 800, 700);
        let candidates = vec![
            (1u64, current),
            (2, Rect::new(800, 320, 1000, 520)),
            (3, Rect::new(1100, 320, 1300, 520)),
            (4, Rect::new(600, 800, 800, 1000)),
        ];
        assert_eq!(
            find_directional(current, &candidates, NavDir::Right),
            Some(2)
        );
        assert_eq!(
            find_directional(current, &candidates, NavDir::Down),
            Some(4)
        );
        assert_eq!(find_directional(current, &candidates, NavDir::Left), None);
    }

    #[test]
    fn stack_navigation_wraps() {
        assert_eq!(find_next_in_stack(&[1, 2, 3], 3), Some(1));
        assert_eq!(find_next_in_stack(&[1, 2, 3], 1), Some(2));
        assert_eq!(find_next_in_stack(&[7], 7), None);
        assert_eq!(find_next_in_stack(&[], 1), None);
        // Current not in list starts at the top.
        assert_eq!(find_next_in_stack(&[1, 2, 3], 9), Some(1));
    }
}
