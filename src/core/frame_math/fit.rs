//! Clamp a resolved frame to the window's min/max limits.

use crate::core::actions::WindowAction;
use crate::core::rect::Rect;

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
