//! Zone frame for halves, quarters, thirds, fourths and centered strips.

use crate::core::actions::WindowAction;
use crate::core::rect::Rect;

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
}
