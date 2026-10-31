//! Single-axis maximize: stretch one dimension across the work area.

use crate::core::rect::Rect;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
