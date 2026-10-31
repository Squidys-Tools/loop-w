//! Center the current frame inside the work area.

use crate::core::rect::Rect;

/// Center the current size inside the work area.
pub fn center_frame(work: Rect, current: Rect) -> Rect {
    let width = current.width().min(work.width());
    let height = current.height().min(work.height());
    let left = work.left + (work.width() - width) / 2;
    let top = work.top + (work.height() - height) / 2;
    Rect::new(left, top, left + width, top + height)
}
