//! Window frame math: zones, axis maximize, fill, center, nudge, fit.
//!
//! Direct port of `WindowFrameMath`. All functions are pure and DPI aware
//! through explicit parameters so they stay unit testable.

use super::actions::WindowAction;
use super::rect::Rect;

/// Zone frame for halves, quarters, thirds, fourths and centered strips.
pub fn zone_frame(work: Rect, action: WindowAction) -> Rect {
    let (left, top, right, bottom) = (work.left, work.top, work.right, work.bottom);
    let mid_x = left + work.width() / 2;
    let mid_y = top + work.height() / 2;
    let third_w = work.width() / 3;
    let two_third_w = work.width() * 2 / 3;
    let third_h = work.height() / 3;
    let two_third_h = work.height() * 2 / 3;
    let r = Rect::new;
    match action {
        WindowAction::LeftHalf => r(left, top, mid_x, bottom),
        WindowAction::RightHalf => r(mid_x, top, right, bottom),
        WindowAction::TopHalf => r(left, top, right, mid_y),
        WindowAction::BottomHalf => r(left, mid_y, right, bottom),
        WindowAction::TopLeftQuarter => r(left, top, mid_x, mid_y),
        WindowAction::TopRightQuarter => r(mid_x, top, right, mid_y),
        WindowAction::BottomLeftQuarter => r(left, mid_y, mid_x, bottom),
        WindowAction::BottomRightQuarter => r(mid_x, mid_y, right, bottom),
        WindowAction::HorizontalCenterHalf => r(
            left + work.width() / 4,
            top,
            left + work.width() * 3 / 4,
            bottom,
        ),
        WindowAction::VerticalCenterHalf => r(
            left,
            top + work.height() / 4,
            right,
            top + work.height() * 3 / 4,
        ),
        WindowAction::LeftThird => r(left, top, left + third_w, bottom),
        WindowAction::LeftTwoThirds => r(left, top, left + two_third_w, bottom),
        WindowAction::HorizontalCenterThird => r(left + third_w, top, left + two_third_w, bottom),
        WindowAction::RightTwoThirds => r(left + third_w, top, right, bottom),
        WindowAction::RightThird => r(left + two_third_w, top, right, bottom),
        WindowAction::TopThird => r(left, top, right, top + third_h),
        WindowAction::TopTwoThirds => r(left, top, right, top + two_third_h),
        WindowAction::VerticalCenterThird => r(left, top + third_h, right, top + two_third_h),
        WindowAction::BottomTwoThirds => r(left, top + third_h, right, bottom),
        WindowAction::BottomThird => r(left, top + two_third_h, right, bottom),
        WindowAction::FirstFourth => r(left, top, left + work.width() / 4, bottom),
        WindowAction::SecondFourth => r(
            left + work.width() / 4,
            top,
            left + work.width() / 2,
            bottom,
        ),
        WindowAction::ThirdFourth => r(
            left + work.width() / 2,
            top,
            left + work.width() * 3 / 4,
            bottom,
        ),
        WindowAction::FourthFourth => r(left + work.width() * 3 / 4, top, right, bottom),
        WindowAction::LeftThreeFourths => r(left, top, left + work.width() * 3 / 4, bottom),
        WindowAction::RightThreeFourths => r(left + work.width() / 4, top, right, bottom),
        _ => work,
    }
}

/// Keep the current width, stretch vertically across the work area.
pub fn maximize_height_frame(work: Rect, current: Rect) -> Rect {
    let width = current.width().min(work.width());
    let left = work.left.max(current.left.min(work.right - width));
    Rect::new(left, work.top, left + width, work.bottom)
}

/// Keep the current height, stretch horizontally across the work area.
pub fn maximize_width_frame(work: Rect, current: Rect) -> Rect {
    let height = current.height().min(work.height());
    let top = work.top.max(current.top.min(work.bottom - height));
    Rect::new(work.left, top, work.right, top + height)
}

/// Largest obstacle-free rectangle inside `work` containing headroom logic.
///
/// Ports the C# candidate-boundary search: candidate edges come from the work
/// area, the current frame, and obstacle-projected bounds.
pub fn fill_available_frame(work: Rect, current: Rect, obstacles: &[Rect]) -> Rect {
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

/// Center the current size inside the work area.
pub fn center_frame(work: Rect, current: Rect) -> Rect {
    let width = current.width().min(work.width());
    let height = current.height().min(work.height());
    let left = work.left + (work.width() - width) / 2;
    let top = work.top + (work.height() - height) / 2;
    Rect::new(left, top, left + width, top + height)
}

/// Grow/shrink/nudge the current frame. `dpi_scale` scales the 48px step.
pub fn manipulate_frame(work: Rect, action: WindowAction, current: Rect, dpi_scale: f64) -> Rect {
    let _ = work;
    let step = (48.0 * dpi_scale).round() as i32;
    let (mut width, mut height) = (current.width(), current.height());
    let r = Rect::new;
    match action {
        WindowAction::Larger
        | WindowAction::Smaller
        | WindowAction::ScaleUp
        | WindowAction::ScaleDown => {
            let (step_w, step_h) = (width.max(32) / 10.max(32), height.max(32) / 10.max(32));
            // Note: mirrors C# `Math.Max(32, dim / 10)`.
            let (step_w, step_h) = (step_w.max(32), step_h.max(32));
            match action {
                WindowAction::ScaleUp | WindowAction::ScaleDown => {
                    let scale = if action == WindowAction::ScaleUp {
                        1.1
                    } else {
                        0.9
                    };
                    width = (width as f64 * scale).round() as i32;
                    height = (height as f64 * scale).round() as i32;
                }
                _ => {
                    width = if action == WindowAction::Larger {
                        width + step_w
                    } else {
                        width - step_w
                    };
                    height = if action == WindowAction::Larger {
                        height + step_h
                    } else {
                        height - step_h
                    };
                }
            }
            width = width.max(step);
            height = height.max(step);
            let cx = current.left + current.width() / 2;
            let cy = current.top + current.height() / 2;
            r(
                cx - width / 2,
                cy - height / 2,
                cx - width / 2 + width,
                cy - height / 2 + height,
            )
        }
        WindowAction::GrowLeft => r(
            current.left - step,
            current.top,
            current.right,
            current.bottom,
        ),
        WindowAction::GrowRight => r(
            current.left,
            current.top,
            current.right + step,
            current.bottom,
        ),
        WindowAction::GrowTop => r(
            current.left,
            current.top - step,
            current.right,
            current.bottom,
        ),
        WindowAction::GrowBottom => r(
            current.left,
            current.top,
            current.right,
            current.bottom + step,
        ),
        WindowAction::GrowHorizontal => r(
            current.left - step,
            current.top,
            current.right + step,
            current.bottom,
        ),
        WindowAction::GrowVertical => r(
            current.left,
            current.top - step,
            current.right,
            current.bottom + step,
        ),
        WindowAction::ShrinkLeft => r(
            (current.right - 1).min(current.left + step),
            current.top,
            current.right,
            current.bottom,
        ),
        WindowAction::ShrinkRight => r(
            current.left,
            current.top,
            (current.left + 1).max(current.right - step),
            current.bottom,
        ),
        WindowAction::ShrinkTop => r(
            current.left,
            (current.bottom - 1).min(current.top + step),
            current.right,
            current.bottom,
        ),
        WindowAction::ShrinkBottom => r(
            current.left,
            current.top,
            current.right,
            (current.top + 1).max(current.bottom - step),
        ),
        WindowAction::ShrinkHorizontal => {
            let new_width = (current.width() - 2 * step).max(1);
            let cx = current.left + current.width() / 2;
            r(
                cx - new_width / 2,
                current.top,
                cx - new_width / 2 + new_width,
                current.bottom,
            )
        }
        WindowAction::ShrinkVertical => {
            let new_height = (current.height() - 2 * step).max(1);
            let cy = current.top + current.height() / 2;
            r(
                current.left,
                cy - new_height / 2,
                current.right,
                cy - new_height / 2 + new_height,
            )
        }
        WindowAction::MoveLeft => r(
            current.left - step,
            current.top,
            current.right - step,
            current.bottom,
        ),
        WindowAction::MoveRight => r(
            current.left + step,
            current.top,
            current.right + step,
            current.bottom,
        ),
        WindowAction::MoveUp => r(
            current.left,
            current.top - step,
            current.right,
            current.bottom - step,
        ),
        WindowAction::MoveDown => r(
            current.left,
            current.top + step,
            current.right,
            current.bottom + step,
        ),
        _ => current,
    }
}

/// Window size limits (from `WM_GETMINMAXINFO`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinMaxLimits {
    pub min_w: i32,
    pub min_h: i32,
    pub max_w: i32,
    pub max_h: i32,
}

/// Clamp a frame to its limits and keep the expected edge anchored.
pub fn fit_frame(bounds: Rect, action: WindowAction, frame: Rect, limits: MinMaxLimits) -> Rect {
    let mut width = frame.width().max(limits.min_w);
    let mut height = frame.height().max(limits.min_h);
    if limits.max_w > 0 {
        width = width.min(limits.max_w);
    }
    if limits.max_h > 0 {
        height = height.min(limits.max_h);
    }
    width = width.min(bounds.width());
    height = height.min(bounds.height());

    if is_centered_action(action) {
        let center_left = frame.left + (frame.width() - width) / 2;
        let center_top = frame.top + (frame.height() - height) / 2;
        let left = bounds.left.max(center_left.min(bounds.right - width));
        let top = bounds.top.max(center_top.min(bounds.bottom - height));
        return Rect::new(left, top, left + width, top + height);
    }

    let mut left = bounds.left.max(frame.left.min(bounds.right - width));
    let mut top = bounds.top.max(frame.top.min(bounds.bottom - height));
    if touches_right(action) {
        left = bounds.right - width;
    }
    if touches_bottom(action) {
        top = bounds.bottom - height;
    }
    if touches_left(action) {
        left = bounds.left;
    }
    if touches_top(action) {
        top = bounds.top;
    }
    Rect::new(left, top, left + width, top + height)
}

fn is_centered_action(action: WindowAction) -> bool {
    use WindowAction::*;
    matches!(
        action,
        Center
            | AlmostMaximize
            | HorizontalCenterHalf
            | VerticalCenterHalf
            | HorizontalCenterThird
            | VerticalCenterThird
            | Larger
            | Smaller
            | ScaleUp
            | ScaleDown
            | GrowHorizontal
            | GrowVertical
            | ShrinkHorizontal
            | ShrinkVertical
    )
}

fn touches_left(action: WindowAction) -> bool {
    use WindowAction::*;
    matches!(
        action,
        LeftHalf
            | TopHalf
            | BottomHalf
            | TopLeftQuarter
            | BottomLeftQuarter
            | LeftThird
            | LeftTwoThirds
            | FirstFourth
            | LeftThreeFourths
            | TopThird
            | TopTwoThirds
            | BottomTwoThirds
            | BottomThird
            | MaximizeWidth
            | Maximize
            | Fullscreen
    )
}

fn touches_right(action: WindowAction) -> bool {
    use WindowAction::*;
    matches!(
        action,
        RightHalf
            | TopHalf
            | BottomHalf
            | TopRightQuarter
            | BottomRightQuarter
            | RightThird
            | RightTwoThirds
            | FourthFourth
            | RightThreeFourths
            | TopThird
            | TopTwoThirds
            | BottomTwoThirds
            | BottomThird
            | MaximizeWidth
            | Maximize
            | Fullscreen
    )
}

fn touches_top(action: WindowAction) -> bool {
    use WindowAction::*;
    matches!(
        action,
        TopHalf
            | TopLeftQuarter
            | TopRightQuarter
            | TopThird
            | TopTwoThirds
            | MaximizeHeight
            | FirstFourth
            | SecondFourth
            | ThirdFourth
            | FourthFourth
            | LeftThreeFourths
            | RightThreeFourths
            | Maximize
            | Fullscreen
    )
}

fn touches_bottom(action: WindowAction) -> bool {
    use WindowAction::*;
    matches!(
        action,
        BottomHalf
            | BottomLeftQuarter
            | BottomRightQuarter
            | BottomThird
            | BottomTwoThirds
            | MaximizeHeight
            | FirstFourth
            | SecondFourth
            | ThirdFourth
            | FourthFourth
            | LeftThreeFourths
            | RightThreeFourths
            | Maximize
            | Fullscreen
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_and_quarters_cover_work_area() {
        let work = Rect::new(0, 0, 1200, 800);
        assert_eq!(
            zone_frame(work, WindowAction::LeftHalf),
            Rect::new(0, 0, 600, 800)
        );
        assert_eq!(
            zone_frame(work, WindowAction::RightHalf),
            Rect::new(600, 0, 1200, 800)
        );
        assert_eq!(
            zone_frame(work, WindowAction::TopLeftQuarter),
            Rect::new(0, 0, 600, 400)
        );
        assert_eq!(
            zone_frame(work, WindowAction::BottomRightQuarter),
            Rect::new(600, 400, 1200, 800)
        );
    }

    #[test]
    fn thirds_tile_without_gaps() {
        let work = Rect::new(0, 0, 1200, 900);
        let left = zone_frame(work, WindowAction::LeftThird);
        let center = zone_frame(work, WindowAction::HorizontalCenterThird);
        let right = zone_frame(work, WindowAction::RightThird);
        assert_eq!(
            (left.width(), center.width(), right.width()),
            (400, 400, 400)
        );
        assert_eq!(left.right, center.left);
        assert_eq!(center.right, right.left);
        assert_eq!(right.right, work.right);
    }

    #[test]
    fn axis_maximize_preserves_other_dimension() {
        let work = Rect::new(0, 0, 1920, 1040);
        let current = Rect::new(100, 100, 500, 400);
        assert_eq!(
            maximize_height_frame(work, current),
            Rect::new(100, 0, 500, 1040)
        );
        assert_eq!(
            maximize_width_frame(work, current),
            Rect::new(0, 100, 1920, 400)
        );
    }

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
    fn fit_clamps_and_anchors_right_edge() {
        let bounds = Rect::new(0, 0, 1000, 800);
        let frame = Rect::new(800, 0, 1400, 800);
        let limits = MinMaxLimits {
            min_w: 100,
            min_h: 100,
            max_w: 0,
            max_h: 0,
        };
        let fitted = fit_frame(bounds, WindowAction::RightHalf, frame, limits);
        assert_eq!(fitted.right, bounds.right);
        assert!(fitted.width() <= bounds.width());
    }
}
