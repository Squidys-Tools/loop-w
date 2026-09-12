//! Ideal-frame computation for every window action.
//!
//! Ports `TryGetTargetFrame` + screen-frame helpers. Pure geometry delegates
//! to `crate::core::frame_math`; monitor/DPI/current-rect reads are live.

use super::monitor_service;
use super::native;
use super::policy;
use std::sync::{Mutex, OnceLock};

use crate::core::actions::WindowAction;
use crate::core::frame_math;
use crate::core::monitor::{translate_frame, MonitorMoveSizePolicy, MonitorSnapshot};
use crate::core::rect::Rect;

#[derive(Clone)]
struct CacheEntry {
    hwnd: u64,
    action: WindowAction,
    generation: u64,
    current: Rect,
    result: Result<Rect, String>,
}

static CACHE: OnceLock<Mutex<Option<CacheEntry>>> = OnceLock::new();

fn cache() -> &'static Mutex<Option<CacheEntry>> {
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Clear preview geometry after monitor or padding settings change.
pub fn invalidate_cache() {
    if let Ok(mut entry) = cache().lock() {
        *entry = None;
    }
}

fn cacheable(action: WindowAction) -> bool {
    // This action depends on every other top-level window and must observe
    // their live movement instead of reusing a frame from the prior tick.
    action != WindowAction::FillAvailableSpace
}

/// Ideal (pre-clamp) frame for `action` on `hwnd`.
pub fn target_frame(hwnd: u64, action: WindowAction) -> Result<Rect, String> {
    if hwnd == 0 {
        return Err("The target window is no longer available.".to_string());
    }
    let native_hwnd = native::from_raw(hwnd as isize);
    let current = native::window_rect(native_hwnd)
        .ok_or_else(|| "The target window is no longer available.".to_string())?;
    let generation = monitor_service::generation();
    if cacheable(action) {
        if let Ok(entry) = cache().lock() {
            if let Some(entry) = entry.as_ref().filter(|entry| {
                entry.hwnd == hwnd
                    && entry.action == action
                    && entry.generation == generation
                    && entry.current == current
            }) {
                return entry.result.clone();
            }
        }
    }
    if let Err(diagnostic) = policy::try_authorize_action(hwnd, action) {
        return Err(diagnostic.to_string());
    }
    let snapshot = monitor_service::for_window(hwnd)
        .ok_or_else(|| "Could not determine the target monitor.".to_string())?;
    let work = snapshot.work;
    let monitor_rect = snapshot.monitor;
    let result = match action {
        WindowAction::Maximize => Ok(work),
        WindowAction::Fullscreen => Ok(monitor_rect),
        WindowAction::MaximizeHeight => Ok(frame_math::maximize_height_frame(work, current)),
        WindowAction::MaximizeWidth => Ok(frame_math::maximize_width_frame(work, current)),
        WindowAction::FillAvailableSpace => {
            let obstacles = super::query::layout_frames(hwnd);
            Ok(frame_math::fill_available_frame(work, current, &obstacles))
        }
        WindowAction::Center => Ok(frame_math::center_frame(work, current)),
        WindowAction::AlmostMaximize => {
            let margin = (12.0 * native::dpi_scale_for_window(native::from_raw(hwnd as isize)))
                .round() as i32;
            Ok(Rect::new(
                work.left + margin,
                work.top + margin,
                work.right - margin,
                work.bottom - margin,
            ))
        }
        WindowAction::NextScreen | WindowAction::PreviousScreen => {
            screen_frame(hwnd, action, snapshot)
        }
        WindowAction::LeftScreen
        | WindowAction::RightScreen
        | WindowAction::TopScreen
        | WindowAction::BottomScreen => directional_screen_frame(action, snapshot, current),
        WindowAction::Larger
        | WindowAction::Smaller
        | WindowAction::ScaleUp
        | WindowAction::ScaleDown
        | WindowAction::GrowLeft
        | WindowAction::GrowRight
        | WindowAction::GrowTop
        | WindowAction::GrowBottom
        | WindowAction::GrowHorizontal
        | WindowAction::GrowVertical
        | WindowAction::ShrinkLeft
        | WindowAction::ShrinkRight
        | WindowAction::ShrinkTop
        | WindowAction::ShrinkBottom
        | WindowAction::ShrinkHorizontal
        | WindowAction::ShrinkVertical
        | WindowAction::MoveLeft
        | WindowAction::MoveRight
        | WindowAction::MoveUp
        | WindowAction::MoveDown => Ok(frame_math::manipulate_frame(
            work,
            action,
            current,
            native::dpi_scale_for_window(native::from_raw(hwnd as isize)),
        )),
        _ if is_zone(action) => Ok(frame_math::zone_frame(work, action)),
        _ => Err(format!("Unsupported action: {}", action.display_name())),
    };
    if cacheable(action) {
        if let Ok(mut entry) = cache().lock() {
            *entry = Some(CacheEntry {
                hwnd,
                action,
                generation,
                current,
                result: result.clone(),
            });
        }
    }
    result
}

fn is_zone(action: WindowAction) -> bool {
    use WindowAction as A;
    matches!(
        action,
        A::LeftHalf
            | A::RightHalf
            | A::TopHalf
            | A::BottomHalf
            | A::TopLeftQuarter
            | A::TopRightQuarter
            | A::BottomLeftQuarter
            | A::BottomRightQuarter
            | A::HorizontalCenterHalf
            | A::VerticalCenterHalf
            | A::FirstFourth
            | A::SecondFourth
            | A::ThirdFourth
            | A::FourthFourth
            | A::LeftThreeFourths
            | A::RightThreeFourths
            | A::LeftThird
            | A::LeftTwoThirds
            | A::HorizontalCenterThird
            | A::RightTwoThirds
            | A::RightThird
            | A::TopThird
            | A::TopTwoThirds
            | A::VerticalCenterThird
            | A::BottomTwoThirds
            | A::BottomThird
    )
}

fn policy() -> MonitorMoveSizePolicy {
    monitor_service::move_size_policy()
}

fn screen_frame(
    hwnd: u64,
    action: WindowAction,
    current_snapshot: MonitorSnapshot,
) -> Result<Rect, String> {
    let monitors = monitor_service::all();
    if monitors.len() < 2 {
        return Err("Only one monitor is connected.".to_string());
    }
    let index = monitors
        .iter()
        .position(|snapshot| rects_close(snapshot.work, current_snapshot.work))
        .unwrap_or(0);
    let next = match action {
        WindowAction::NextScreen => &monitors[(index + 1) % monitors.len()],
        _ => &monitors[(index + monitors.len() - 1) % monitors.len()],
    };
    let current =
        native::window_rect(native::from_raw(hwnd as isize)).unwrap_or(Rect::new(0, 0, 0, 0));
    Ok(translate_frame(current, current_snapshot, *next, policy()))
}

fn directional_screen_frame(
    action: WindowAction,
    current_snapshot: MonitorSnapshot,
    current: Rect,
) -> Result<Rect, String> {
    let (dir_x, dir_y) = match action {
        WindowAction::LeftScreen => (-1i64, 0i64),
        WindowAction::RightScreen => (1, 0),
        WindowAction::TopScreen => (0, -1),
        _ => (0, 1),
    };
    let (cur_cx, cur_cy) = (
        current_snapshot.work.left + current_snapshot.work.width() / 2,
        current_snapshot.work.top + current_snapshot.work.height() / 2,
    );
    let mut best: Option<&MonitorSnapshot> = None;
    let mut best_score = 0i64;
    let monitors = monitor_service::all();
    for snapshot in &monitors {
        if rects_close(snapshot.work, current_snapshot.work) {
            continue;
        }
        let cand_cx = snapshot.work.left + snapshot.work.width() / 2;
        let cand_cy = snapshot.work.top + snapshot.work.height() / 2;
        let score =
            (cand_cx as i64 - cur_cx as i64) * dir_x + (cand_cy as i64 - cur_cy as i64) * dir_y;
        if score > 0 && score > best_score {
            best_score = score;
            best = Some(snapshot);
        }
    }
    let Some(target) = best else {
        return Err("No monitor in that direction.".to_string());
    };
    Ok(translate_frame(
        current,
        current_snapshot,
        *target,
        policy(),
    ))
}

fn rects_close(first: Rect, second: Rect) -> bool {
    (first.left - second.left).abs() <= 2
        && (first.top - second.top).abs() <= 2
        && (first.right - second.right).abs() <= 2
        && (first.bottom - second.bottom).abs() <= 2
}
