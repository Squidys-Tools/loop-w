//! Title-bar drag snapping runtime (edge preview + commit/cancel).
//!
//! Ports `DragSnapService` staging: threshold checks are pure
//! (`crate::core::drag_snap`); this module tracks the pre-drag frame so a
//! cancelled snap can restore it when the setting is enabled.

use crate::core::rect::Rect;

/// In-flight drag state for one window.
#[derive(Debug, Default)]
pub struct DragState {
    pub window: u64,
    pub pre_drag_frame: Option<Rect>,
    pub active: bool,
}

impl DragState {
    pub fn begin(&mut self, window: u64, frame: Rect) {
        self.window = window;
        self.pre_drag_frame = Some(frame);
        self.active = true;
    }

    pub fn cancel(&mut self) -> Option<Rect> {
        self.active = false;
        self.pre_drag_frame.take()
    }
}
