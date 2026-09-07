//! Title-bar drag snapping: detect, preview, commit or restore.
//!
//! Ports `DragSnapService`. Button down/up arrives from the mouse hook;
//! move tracking is driven once per UI frame (`track`) while a drag is
//! active, which subsumes the C# move events + 45 ms watchdog without
//! flooding the event queue. Nothing moves until button-up.

use std::sync::{Mutex, OnceLock};

use windows::Win32::UI::WindowsAndMessaging::*;

use super::native;
use crate::core::actions::WindowAction;
use crate::core::drag_snap::try_resolve;
use crate::core::frame_math::zone_frame;
use crate::core::rect::{Point, Rect};

const DRAG_START_DISTANCE_SQ: i32 = 16; // 4 px, squared
const HTCAPTION: usize = 2;
const VK_LBUTTON: i32 = 0x01;

/// What the preview window should do after a track step.
#[derive(Debug, Clone, PartialEq)]
pub enum SnapTrack {
    Idle,
    Show(Rect),
    Update(Rect),
    Hide,
}

/// How a finished gesture resolves (applied by the UI thread).
/// `None` (no variant) means the gesture evaporates: plain click or
/// interior-only drag with no zone contact and no restore warranted.
#[derive(Debug, Clone)]
pub enum SnapFinish {
    Apply {
        window: u64,
        action: WindowAction,
        frame: Rect,
    },
    Restore {
        window: u64,
        frame: Rect,
    },
}

struct SnapState {
    window: u64,
    original_frame: Rect,
    start_point: Point,
    dragging: bool,
    target: Option<(WindowAction, Rect)>,
    had_candidate: bool,
}

static STATE: OnceLock<Mutex<Option<SnapState>>> = OnceLock::new();

fn state() -> &'static Mutex<Option<SnapState>> {
    STATE.get_or_init(|| Mutex::new(None))
}

/// Hook-thread input: physical left-button transitions.
pub fn note_button(down: bool) {
    if down {
        if let Some(cursor) = native::cursor_pos() {
            super::events::push(super::events::RuntimeEvent::SnapBegin {
                window: 0, // resolved on the UI thread (hit-test may block)
                frame: Rect::new(0, 0, 0, 0),
                cursor,
            });
        }
    } else {
        super::events::push(super::events::RuntimeEvent::SnapEnd { released: true });
    }
}

/// UI-thread drag start: caption hit-test + eligibility gates.
pub fn begin_at_cursor(cursor: Point) {
    if !super::shared::snapshot().drag_snap_enabled {
        return;
    }
    if state().lock().map(|s| s.is_some()).unwrap_or(true) {
        return;
    }
    let hwnd = native::ancestor_root(native::window_from_point(cursor));
    if hwnd.is_invalid() {
        return;
    }
    let raw = native::raw(hwnd) as u64;
    if !super::query::is_eligible_for_snap(raw) || native::is_zoomed(hwnd) {
        return;
    }
    if (native::window_style(hwnd) as u32 & WS_CAPTION.0) == 0 {
        return;
    }
    if native::nc_hit_test(hwnd, cursor) != Some(HTCAPTION) {
        return;
    }
    let Some(frame) = native::window_rect(hwnd) else {
        return;
    };
    if let Ok(mut slot) = state().lock() {
        *slot = Some(SnapState {
            window: raw,
            original_frame: frame,
            start_point: cursor,
            dragging: false,
            target: None,
            had_candidate: false,
        });
    }
}

/// Per-frame tracking. Returns preview action + optional finished gesture.
/// The watchdog is folded in: a physically-released button or a dead window
/// resolves here as a cancel when no SnapEnd event arrives first.
pub fn track(cursor: Point) -> (SnapTrack, Option<SnapFinish>) {
    let Ok(mut slot) = state().lock() else {
        return (SnapTrack::Idle, None);
    };
    let Some(drag) = slot.as_mut() else {
        return (SnapTrack::Idle, None);
    };
    if !native::async_key_down(VK_LBUTTON)
        || !native::is_window(native::from_raw(drag.window as isize))
    {
        // Watchdog: only a drag that actually saw a zone may restore;
        // a plain click ends silently (matches EndGesture's hadCandidate gate).
        let finished = if drag.had_candidate {
            finish_gesture(drag, false)
        } else {
            None
        };
        *slot = None;
        return (SnapTrack::Hide, finished);
    }
    let dx = cursor.x - drag.start_point.x;
    let dy = cursor.y - drag.start_point.y;
    if !drag.dragging {
        if dx * dx + dy * dy < DRAG_START_DISTANCE_SQ {
            return (SnapTrack::Idle, None);
        }
        drag.dragging = true;
    }
    let settings = super::shared::snapshot();
    let target = super::monitor_service::for_point(cursor).and_then(|snapshot| {
        try_resolve(
            snapshot.monitor,
            snapshot.work,
            cursor,
            settings.drag_snap_threshold,
        )
        .map(|zone| {
            let action = zone.action();
            (action, zone_frame(snapshot.work, action))
        })
    });
    let had_previous = drag.target.is_some();
    if target.map(|(_, frame)| frame) == drag.target.map(|(_, frame)| frame) {
        return (SnapTrack::Idle, None);
    }
    drag.target = target;
    match target {
        None => (SnapTrack::Hide, None),
        Some((_, frame)) => {
            drag.had_candidate = true;
            if had_previous {
                (SnapTrack::Update(frame), None)
            } else {
                (SnapTrack::Show(frame), None)
            }
        }
    }
}

/// Button-up resolution from a SnapEnd event.
pub fn end_released() -> Option<SnapFinish> {
    let mut slot = state().lock().ok()?;
    let drag = slot.take()?;
    if !drag.dragging || !drag.had_candidate {
        return None;
    }
    finish_gesture(&drag, true)
}

fn finish_gesture(drag: &SnapState, released: bool) -> Option<SnapFinish> {
    let commit = released && drag.target.is_some();
    match drag.target {
        Some((action, frame)) if commit => Some(SnapFinish::Apply {
            window: drag.window,
            action,
            frame,
        }),
        _ => {
            if super::shared::snapshot().restore_pre_drag_on_cancel {
                Some(SnapFinish::Restore {
                    window: drag.window,
                    frame: drag.original_frame,
                })
            } else {
                None
            }
        }
    }
}

/// Current live target for preview refresh (display-change path).
pub fn current_target() -> Option<(WindowAction, Rect)> {
    state()
        .lock()
        .ok()
        .and_then(|s| s.as_ref().and_then(|d| d.target))
}

/// Whether a drag session is in flight (for gating SnapEnd handling).
pub fn is_active() -> bool {
    state().lock().map(|s| s.is_some()).unwrap_or(false)
}

/// Abort any in-flight gesture.
/// Returns the cancel resolution (restore-if-warranted) so disabling
/// mid-drag ends as Disabled: preview hides and the pre-drag frame comes
/// back when a candidate was seen — never a silent drop.
pub fn disable() -> Option<SnapFinish> {
    let mut slot = state().lock().ok()?;
    let drag = slot.take()?;
    if !drag.dragging || !drag.had_candidate {
        return None;
    }
    finish_gesture(&drag, false)
}
