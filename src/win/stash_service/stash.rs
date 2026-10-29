//! Stashing: park a live window at its nearest work-area edge.

use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::stash::{
    calculate_stashed_frame, nearest_edge, StashEdge, StashMonitor, StashRecord, StashRect,
};
use crate::win::{monitor_service, native, policy, shared};

use super::convert::placement_to_stash;
use super::persist::persist_upsert;
use super::{live, LiveRecord};

/// Stash a window at its nearest work-area edge.
pub fn stash(hwnd: u64) -> Result<String, String> {
    use crate::core::actions::WindowAction;
    if hwnd == 0 || !native::is_window(native::from_raw(hwnd as isize)) {
        return Err("The target window is no longer available.".to_string());
    }
    if let Err(diagnostic) = policy::try_authorize_action(hwnd, WindowAction::Stash) {
        return Err(diagnostic.to_string());
    }
    {
        let guard = live()
            .lock()
            .map_err(|_| "Internal state is unavailable.".to_string())?;
        if guard.by_hwnd.contains_key(&hwnd) {
            return Err("The target window is already stashed.".to_string());
        }
    }
    let native_hwnd = native::from_raw(hwnd as isize);
    let identity = native::window_identity(native_hwnd)
        .ok_or_else(|| "Could not identify the target window.".to_string())?;
    let original = native::window_placement(native_hwnd)
        .ok_or_else(|| "Could not read the target window's placement.".to_string())?;
    let _ = native::window_rect(native_hwnd)
        .ok_or_else(|| "Could not identify the target window.".to_string())?;
    // Collapse maximized/minimized so geometry is measurable.
    native::show_window(native_hwnd, SW_RESTORE);
    let current = native::window_rect(native_hwnd)
        .ok_or_else(|| "Could not identify the target window.".to_string())?;
    let snapshot = monitor_service::for_rect(current)
        .ok_or_else(|| "Could not determine the target monitor.".to_string())?;
    let settings = shared::snapshot();
    let edge = nearest_edge(snapshot.work, current);
    let frame = calculate_stashed_frame(snapshot.work, current, edge, settings.stash_peek);
    if !native::set_pos(native_hwnd, frame) {
        let _ = native::set_placement(native_hwnd, &original);
        return Err(
            "Windows rejected the stash. The target may be elevated, protected, or non-movable."
                .to_string(),
        );
    }
    let can_persist = settings.stash_persistence_enabled
        && !identity.executable_path.trim().is_empty()
        && !identity.window_class.trim().is_empty();
    let persisted_id = if can_persist {
        uuid::Uuid::new_v4().simple().to_string()
    } else {
        String::new()
    };
    let monitor = StashMonitor {
        monitor: StashRect::from(snapshot.monitor),
        work: StashRect::from(snapshot.work),
        dpi_x: snapshot.dpi_x,
        dpi_y: snapshot.dpi_y,
    };
    {
        let mut guard = live()
            .lock()
            .map_err(|_| "Internal state is unavailable.".to_string())?;
        guard.by_hwnd.insert(
            hwnd,
            LiveRecord {
                edge,
                placement: original,
                monitor,
                frame,
                identity: identity.clone(),
                persisted_id: persisted_id.clone(),
            },
        );
        guard.order.retain(|id| *id != hwnd);
        guard.order.push(hwnd);
    }
    if can_persist {
        persist_upsert(&StashRecord {
            id: persisted_id,
            executable_path: identity.executable_path,
            process_id: identity.process_id,
            window_class: identity.window_class,
            title: identity.title,
            edge,
            original_placement: placement_to_stash(&original),
            original_monitor: live_monitor(hwnd),
            stashed_frame: StashRect::from(frame),
        });
    }
    Ok(format!(
        "Stashed the window at the {} edge",
        edge_name(edge)
    ))
}

fn live_monitor(hwnd: u64) -> StashMonitor {
    live()
        .lock()
        .ok()
        .and_then(|guard| guard.by_hwnd.get(&hwnd).map(|r| r.monitor.clone()))
        .unwrap_or_default()
}

fn edge_name(edge: StashEdge) -> &'static str {
    match edge {
        StashEdge::Left => "left",
        StashEdge::Right => "right",
        StashEdge::Top => "top",
        StashEdge::Bottom => "bottom",
    }
}
