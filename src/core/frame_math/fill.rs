//! Largest obstacle-free rectangle inside the work area.

use crate::core::rect::Rect;

/// Largest obstacle-free rectangle inside `work` containing headroom logic.
///
/// Ports the C# candidate-boundary search: candidate edges come from the work
/// area, the current frame, and obstacle-projected bounds.
pub fn fill_available_frame(work: Rect, current: Rect, obstacles: &[Rect]) -> Rect {
    // With no obstacles, the largest valid rectangle is always the work area
    // itself. Avoid building the temporary relevant-obstacle list and the
    // 36-candidate search on the common fast path.
    if obstacles.is_empty() && !work.is_empty() {
        return work;
    }

    let mut min_x = work.left;
    let mut min_y = work.top;
    let mut max_x = work.right;
    let mut max_y = work.bottom;
    let mut relevant = Vec::new();

    for obstacle in obstacles.iter().copied() {
        if obstacle.intersects(current) {
            continue;
        }
        let clipped = obstacle.intersection(work);
        if clipped.is_empty() {
            continue;
        }
        relevant.push(clipped);
        if clipped.right <= current.left {
            min_x = min_x.max(clipped.right);
        }
        if clipped.bottom <= current.top {
            min_y = min_y.max(clipped.bottom);
        }
        if clipped.left >= current.right {
            max_x = max_x.min(clipped.left);
        }
        if clipped.top >= current.bottom {
            max_y = max_y.min(clipped.top);
        }
    }

    let xs = [
        (min_x, max_x),
        (current.left, max_x),
        (min_x, current.right),
        (current.left, work.right),
        (work.left, current.right),
        (work.left, work.right),
    ];
    let ys = [
        (min_y, max_y),
        (current.top, max_y),
        (min_y, current.bottom),
        (current.top, work.bottom),
        (work.top, current.bottom),
        (work.top, work.bottom),
    ];

    let mut best = current;
    let mut best_area = 0i64;
    for (cl, cr) in xs {
        for (ct, cb) in ys {
            if cr <= cl || cb <= ct {
                continue;
            }
            let candidate = Rect::new(cl, ct, cr, cb);
            if relevant.iter().any(|o| candidate.intersects(*o)) {
                continue;
            }
            let area = candidate.area();
            if area > best_area {
                best = candidate;
                best_area = area;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_avoids_obstacles() {
        let work = Rect::new(0, 0, 1200, 800);
        let current = Rect::new(600, 0, 900, 400);
        let obstacles = vec![Rect::new(0, 0, 600, 800)];
        let filled = fill_available_frame(work, current, &obstacles);
        assert!(filled.left >= 600);
        assert!(!filled.intersects(obstacles[0]) || filled == current);
    }

    #[test]
    fn fill_without_obstacles_returns_work_area() {
        let work = Rect::new(100, 50, 1300, 850);
        let current = Rect::new(-500, -400, -100, -50);
        assert_eq!(fill_available_frame(work, current, &[]), work);
    }
}
