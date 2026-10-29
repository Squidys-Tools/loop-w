use iced::{keyboard, window, Task};

use crate::core::actions::WindowAction;
use crate::core::hotkey::{hotkey_name, TriggerModifierSide};
use crate::core::radial::{index_at_vector, GEOMETRY};
use crate::core::radial_targets::{resolve_slot, RadialTarget, ResolvedKeybind};
use crate::core::rect::{Point, Rect};
use crate::win::{self, native};

use super::state::{PreviewFrameCache, PreviewSession, RadialSession};
use super::{Message, State};

pub(in crate::ui) fn open_radial(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    target_hwnd: u64,
) {
    let Some(cursor) = native::cursor_pos() else {
        return;
    };
    let settings = &state.settings;
    let outer = settings.radial_outer_radius;
    let inner = settings.radial_inner_radius;
    let keybinds: Vec<ResolvedKeybind> = settings
        .keybinds
        .iter()
        .map(|bind| ResolvedKeybind {
            id: bind.id.clone(),
            action: bind.action,
            label: hotkey_name(bind.modifiers, bind.vk, TriggerModifierSide::Any),
        })
        .collect();
    let targets: Vec<RadialTarget> = settings
        .radial_slots
        .iter()
        .map(|slot| resolve_slot(slot, &keybinds))
        .collect();
    let center_target = resolve_slot(&settings.center_target, &keybinds);
    let bounds = win::overlay::radial_bounds(cursor, outer);
    // iced positions/sizes in logical pixels: convert once at open.
    let scale = native::dpi_scale_at_point(cursor);
    let (lx, ly, lw, lh) = win::overlay::to_logical(bounds, scale);
    let (id, task) = window::open(crate::ui::windows::radial_settings(lx, ly, lw, lh));
    if let Some(preview) = state.preview.as_mut() {
        // A radial can be opened while a drag preview is already alive. That
        // creates a new z-order relationship which needs one settling pass.
        preview.order_tries = 0;
    }
    let preview_cache = (0..targets.len()).map(|_| None).collect();
    state.radial = Some(RadialSession {
        id,
        target_hwnd,
        center: cursor,
        outer,
        inner,
        hovered: None,
        targets,
        center_target,
        patch_tries: 0,
        window_size: (lw, lh),
        scale,
        preview_cache,
    });
    tasks.push(task.map(Message::RadialOpened));
}

pub(in crate::ui) fn radial_expected(session: &RadialSession) -> Rect {
    win::overlay::radial_bounds(session.center, session.outer)
}

fn wedge_at(session: &RadialSession, cursor: Point) -> Option<usize> {
    if !state_cursor_interaction() {
        return session.hovered;
    }
    wedge_index_at(session.center, session.inner, cursor)
}

/// Resolve a radial selection from the ray through the menu center.
///
/// The menu's outer edge is visual only: once the cursor has left the dead
/// zone, its distance from the center must not change which wedge is active.
fn wedge_index_at(center: Point, inner: f64, cursor: Point) -> Option<usize> {
    let dx = cursor.x as f64 - center.x as f64;
    let dy = cursor.y as f64 - center.y as f64;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < inner {
        return None;
    }
    Some(index_at_vector(dx, dy))
}

fn state_cursor_interaction() -> bool {
    win::shared::cursor_interaction_enabled()
}

pub(in crate::ui) fn update_radial_hover(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    cursor: Point,
) {
    let changed = match state.radial.as_mut() {
        Some(session) => {
            let hovered = wedge_at(session, cursor);
            let changed = session.hovered != hovered;
            session.hovered = hovered;
            changed
        }
        None => return,
    };
    // Target-frame resolution crosses into Win32 and can be relatively
    // expensive. Re-resolve only when the ray selects a different wedge, or
    // when an external event (display/settings change) left the preview gone.
    if changed || state.preview.is_none() {
        refresh_preview_for_hover(state, tasks);
    }
}

fn refresh_preview_for_hover(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if !state.settings.preview_enabled {
        close_preview(state, tasks);
        return;
    }
    let frame = state.radial.as_mut().and_then(radial_preview_frame);
    match frame {
        Some(frame) => ensure_preview(state, tasks, frame),
        None => close_preview(state, tasks),
    }
}

fn radial_preview_frame(session: &mut RadialSession) -> Option<Rect> {
    let index = session.hovered?;
    let target = session.targets.get(index)?;
    let action = target.action()?;
    let generation = win::monitor_service::generation();
    cached_preview_frame(
        &mut session.preview_cache,
        index,
        generation,
        action,
        || win::target_frame::target_frame(session.target_hwnd, action),
    )
}

fn cached_preview_frame(
    cache: &mut [Option<PreviewFrameCache>],
    index: usize,
    generation: u64,
    action: WindowAction,
    resolve: impl FnOnce() -> Result<Rect, String>,
) -> Option<Rect> {
    let cacheable = action != WindowAction::FillAvailableSpace;
    if cacheable {
        if let Some(cached) = cache.get(index).and_then(Option::as_ref) {
            if cached.generation == generation {
                return cached.result.as_ref().ok().copied();
            }
        }
    }
    let result = resolve();
    if cacheable {
        if let Some(slot) = cache.get_mut(index) {
            *slot = Some(PreviewFrameCache {
                generation,
                result: result.clone(),
            });
        }
    }
    result.ok()
}

pub(in crate::ui) fn ensure_preview(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    frame: Rect,
) {
    if frame.width() <= 0 || frame.height() <= 0 {
        // Never leave the last valid preview visible when the producer hands
        // us an invalid/stale frame during a drag or display transition.
        close_preview(state, tasks);
        return;
    }
    let overlay_frame = preview_overlay_frame(frame);
    if let Some(session) = state.preview.as_ref() {
        if session.frame == frame {
            return;
        }
        // Most hover transitions stay on one monitor. Keep the HWND and its
        // iced surface stable in that case. The canvas cache invalidates from
        // the changed target during draw, so do not issue a native same-size
        // resize just to force a redraw.
        if session.overlay_frame == overlay_frame {
            if let Some(session) = state.preview.as_mut() {
                session.frame = frame;
            }
            return;
        }

        let id = session.id;
        // A monitor transition is uncommon and changes the fixed surface's
        // bounds. Update that surface only for the transition itself.
        let scale = native::dpi_scale_at_point(Point::new(overlay_frame.left, overlay_frame.top));
        let (lx, ly, lw, lh) = win::overlay::to_logical(overlay_frame, scale);
        if let Some(session) = state.preview.as_mut() {
            session.frame = frame;
            session.overlay_frame = overlay_frame;
            session.window_size = (lw, lh);
            session.scale = scale;
        }
        // This path is only for a monitor change. Keep the native fallback,
        // but never use it for ordinary quadrant/half transitions.
        if !win::overlay::set_preview_frame(overlay_frame) {
            // The HWND can briefly be unavailable while an open/close is
            // settling. Keep the logical fallback for that short window.
            tasks.push(window::move_to(id, iced::Point::new(lx, ly)));
            tasks.push(window::resize(id, iced::Size::new(lw, lh)));
        }
        return;
    }
    let scale = native::dpi_scale_at_point(Point::new(overlay_frame.left, overlay_frame.top));
    let (lx, ly, lw, lh) = win::overlay::to_logical(overlay_frame, scale);
    let (id, task) = window::open(crate::ui::windows::preview_settings(lx, ly, lw, lh));
    state.preview = Some(PreviewSession {
        id,
        frame,
        overlay_frame,
        patch_tries: 0,
        window_size: (lw, lh),
        scale,
        order_tries: 0,
    });
    tasks.push(task.map(Message::PreviewOpened));
}

/// Keep the preview surface large and stable while the target changes.
///
/// A target zone is always on one monitor, so the monitor frame is sufficient
/// to contain every half/quarter rectangle on that monitor. Using the full
/// monitor (rather than the target frame) means iced never has to resize its
/// canvas during radial hover transitions.
fn preview_overlay_frame(frame: Rect) -> Rect {
    win::monitor_service::for_rect(frame)
        .map(|monitor| monitor.monitor)
        .filter(|monitor| !monitor.is_empty())
        .unwrap_or(frame)
}

pub(in crate::ui) fn close_preview(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if let Some(session) = state.preview.take() {
        tasks.push(window::close(session.id));
    }
}

pub(in crate::ui) fn close_overlays(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if let Some(session) = state.radial.take() {
        tasks.push(window::close(session.id));
    }
    close_preview(state, tasks);
}

pub(in crate::ui) fn commit_radial(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    let Some(session) = state.radial.take() else {
        return;
    };
    tasks.push(window::close(session.id));
    close_preview(state, tasks);
    let target = session
        .hovered
        .and_then(|index| session.targets.get(index).cloned());
    match target {
        Some(RadialTarget::BuiltIn { action, cycle })
        | Some(RadialTarget::Keybind { action, cycle, .. }) => {
            state.status =
                crate::ui::runtime::commit_action(state, session.target_hwnd, action, cycle);
            return;
        }
        _ => {}
    }
    // No wedge hovered: release inside the center hole commits the configured
    // center action; anywhere else cancels without touching the window.
    if let Some(action) = session.center_target.action() {
        if let Some(cursor) = native::cursor_pos() {
            let dx = cursor.x as f64 - session.center.x as f64;
            let dy = cursor.y as f64 - session.center.y as f64;
            if dx * dx + dy * dy < session.inner * session.inner {
                let cycle = session.center_target.cycle_enabled();
                state.status =
                    crate::ui::runtime::commit_action(state, session.target_hwnd, action, cycle);
                return;
            }
        }
    }
    state.status = "Cancelled".to_string();
}

pub(super) fn overlay_key_pressed(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    key: keyboard::Key,
) {
    use iced::keyboard::key::Named;
    if state.radial.is_none() {
        return;
    }
    match key {
        keyboard::Key::Named(Named::Escape) => {
            close_overlays(state, tasks);
            state.status = "Cancelled".to_string();
        }
        keyboard::Key::Named(Named::Enter) => commit_radial(state, tasks),
        keyboard::Key::Named(Named::ArrowLeft) => select_wedge(state, tasks, 4),
        keyboard::Key::Named(Named::ArrowRight) => select_wedge(state, tasks, 0),
        keyboard::Key::Named(Named::ArrowUp) => select_wedge(state, tasks, 6),
        keyboard::Key::Named(Named::ArrowDown) => select_wedge(state, tasks, 2),
        _ => {}
    }
}

fn select_wedge(state: &mut State, tasks: &mut Vec<Task<Message>>, index: usize) {
    if index >= GEOMETRY.len() {
        return;
    }
    if let Some(session) = state.radial.as_mut() {
        session.hovered = Some(index);
    }
    refresh_preview_for_hover(state, tasks);
}

pub(in crate::ui) fn open_tray_menu(
    state: &mut State,
    tasks: &mut Vec<Task<Message>>,
    position: Point,
) {
    if let Some(id) = state.tray_menu.take() {
        tasks.push(window::close(id));
    }
    let (id, task) = window::open(crate::ui::windows::tray_menu_settings(position));
    state.tray_menu = Some(id);
    state.tray_menu_patch_tries = 0;
    tasks.push(task.map(Message::TrayMenuOpened));
}

pub(super) fn close_tray_menu(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    if let Some(id) = state.tray_menu.take() {
        state.tray_menu_patch_tries = 0;
        tasks.push(window::close(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_selection_follows_angle_beyond_outer_edge() {
        let center = Point::new(100, 100);

        assert_eq!(wedge_index_at(center, 40.0, Point::new(900, 100)), Some(0));
        assert_eq!(wedge_index_at(center, 40.0, Point::new(100, -700)), Some(6));
        assert_eq!(wedge_index_at(center, 40.0, Point::new(120, 110)), None);
    }

    #[test]
    fn preview_frame_cache_reuses_wedge_result_until_generation_changes() {
        let mut cache = vec![None];
        let mut resolutions = 0;
        let first = cached_preview_frame(&mut cache, 0, 7, WindowAction::LeftHalf, || {
            resolutions += 1;
            Ok(Rect::new(0, 0, 500, 400))
        });
        let second = cached_preview_frame(&mut cache, 0, 7, WindowAction::LeftHalf, || {
            resolutions += 1;
            Ok(Rect::new(1, 1, 501, 401))
        });

        assert_eq!(first, second);
        assert_eq!(resolutions, 1);

        let refreshed = cached_preview_frame(&mut cache, 0, 8, WindowAction::LeftHalf, || {
            resolutions += 1;
            Ok(Rect::new(2, 2, 502, 402))
        });
        assert_eq!(refreshed, Some(Rect::new(2, 2, 502, 402)));
        assert_eq!(resolutions, 2);
    }

    #[test]
    fn fill_available_preview_always_resolves_again() {
        let mut cache = vec![None];
        let mut resolutions = 0;
        for _ in 0..2 {
            let _ =
                cached_preview_frame(&mut cache, 0, 7, WindowAction::FillAvailableSpace, || {
                    resolutions += 1;
                    Ok(Rect::new(0, 0, 500, 400))
                });
        }
        assert_eq!(resolutions, 2);
        assert!(cache[0].is_none());
    }
}
