use iced::{window, Task};

use crate::ui::{runtime, settings, windows};
use crate::win;

use super::overlays::{close_preview, close_tray_menu, commit_radial, overlay_key_pressed};
use super::{Message, State};

#[allow(clippy::too_many_lines)]
pub(super) fn update(state: &mut State, message: Message) -> Task<Message> {
    let message = match settings::try_update(state, message) {
        Ok(task) => return task,
        Err(message) => message,
    };
    match message {
        Message::FrameTick => runtime::frame_tick(state),
        Message::MainOpened(id) => {
            // `windows::show_main` records the id synchronously when it queues
            // the open task. Ignore a completion from a window that was
            // closed before iced finished creating it; accepting it would
            // resurrect a ghost settings window in resident state.
            if state.main_id != Some(id) {
                return Task::none();
            }
            // Sample the Windows animation flag each time the settings window
            // opens: the user may have changed the OS preference while LoopW
            // stayed tray-resident.
            runtime::refresh_windows_motion(state);
            Task::none()
        }
        Message::RadialOpened(id) => {
            // The id returned by `window::open` is stable. A stale completion
            // must not retarget a newer radial session after close/reopen.
            if state
                .radial
                .as_ref()
                .is_some_and(|session| session.id != id)
            {
                return Task::none();
            }
            Task::none()
        }
        Message::PreviewOpened(id) => {
            // See `RadialOpened`: an old async completion is harmless only if
            // it still belongs to the currently tracked preview session.
            if state
                .preview
                .as_ref()
                .is_some_and(|session| session.id != id)
            {
                return Task::none();
            }
            Task::none()
        }
        Message::TrayMenuOpened(id) => {
            if state.tray_menu != Some(id) {
                return Task::none();
            }
            Task::none()
        }
        Message::WindowClosed(id) => {
            if state.main_id == Some(id) {
                // The settings window opts out of iced's automatic close so
                // the daemon can remain resident. Explicitly close the
                // native window here; clearing only the id leaves its last
                // painted surface behind as a black/ghost window.
                state.main_id = None;
                settings::cancel_captures(state);
                state.confirming_reset_all = false;
                state.color_error = None;
                return window::close(id);
            }
            if state.radial.as_ref().map(|s| s.id) == Some(id) {
                state.radial = None;
                let mut tasks = Vec::new();
                close_preview(state, &mut tasks);
                state.status = "Cancelled".to_string();
                return Task::batch(tasks);
            }
            if state.preview.as_ref().map(|s| s.id) == Some(id) {
                state.preview = None;
            }
            if state.tray_menu == Some(id) {
                state.tray_menu = None;
                state.tray_menu_patch_tries = 0;
                return window::close(id);
            }
            Task::none()
        }
        Message::WindowUnfocused(id) => {
            if state.tray_menu == Some(id) {
                state.tray_menu = None;
                state.tray_menu_patch_tries = 0;
                return window::close(id);
            }
            Task::none()
        }
        Message::TrayMenuSettings => {
            let mut tasks = Vec::new();
            close_tray_menu(state, &mut tasks);
            windows::show_main(state, &mut tasks);
            Task::batch(tasks)
        }
        Message::TrayMenuQuit => {
            let mut tasks = Vec::new();
            close_tray_menu(state, &mut tasks);
            win::events::push(win::events::RuntimeEvent::TrayQuit);
            Task::batch(tasks)
        }
        Message::OverlayKey(key) => {
            let mut tasks = Vec::new();
            overlay_key_pressed(state, &mut tasks, key);
            Task::batch(tasks)
        }
        Message::OverlayCommit => {
            let mut tasks = Vec::new();
            commit_radial(state, &mut tasks);
            Task::batch(tasks)
        }
        Message::RunStartupCommand => {
            let mut tasks = Vec::new();
            runtime::run_startup_command(state, &mut tasks);
            Task::batch(tasks)
        }
        _ => unreachable!("settings message was not handled before runtime dispatch"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::radial_targets::RadialTarget;
    use crate::core::rect::{Point, Rect};
    use crate::ui::app::overlays::open_tray_menu;
    use crate::ui::app::state::{test_state, PreviewSession, RadialSession};

    #[test]
    fn tray_menu_opens_and_closes_when_focus_is_lost() {
        let mut state = test_state();
        let mut tasks = Vec::new();
        open_tray_menu(&mut state, &mut tasks, Point::new(1200, 800));
        let menu = state.tray_menu.expect("tray menu should open");

        let _ = update(&mut state, Message::WindowUnfocused(menu));
        assert_eq!(state.tray_menu, None);
    }

    #[test]
    fn stale_main_open_completion_does_not_resurrect_closed_window() {
        let id = window::Id::unique();
        let mut state = test_state();

        let _ = update(&mut state, Message::MainOpened(id));
        assert_eq!(state.main_id, None);

        state.main_id = Some(id);
        let _ = update(&mut state, Message::WindowClosed(id));
        assert_eq!(state.main_id, None);

        let _ = update(&mut state, Message::MainOpened(id));
        assert_eq!(state.main_id, None);
    }

    #[test]
    fn stale_overlay_open_completion_does_not_retarget_new_session() {
        let current_radial = window::Id::unique();
        let stale_radial = window::Id::unique();
        let current_preview = window::Id::unique();
        let stale_preview = window::Id::unique();
        let mut state = test_state();
        state.radial = Some(RadialSession {
            id: current_radial,
            target_hwnd: 1,
            center: Point::new(100, 100),
            outer: 90.0,
            inner: 50.0,
            hovered: None,
            targets: vec![RadialTarget::None],
            center_target: RadialTarget::None,
            patch_tries: 0,
            window_size: (200.0, 200.0),
            scale: 1.0,
            preview_cache: vec![None],
        });
        state.preview = Some(PreviewSession {
            id: current_preview,
            frame: Rect::new(0, 0, 100, 100),
            overlay_frame: Rect::new(0, 0, 100, 100),
            patch_tries: 0,
            window_size: (100.0, 100.0),
            scale: 1.0,
            order_tries: 0,
        });

        let _ = update(&mut state, Message::RadialOpened(stale_radial));
        let _ = update(&mut state, Message::PreviewOpened(stale_preview));

        assert_eq!(
            state.radial.as_ref().map(|session| session.id),
            Some(current_radial)
        );
        assert_eq!(
            state.preview.as_ref().map(|session| session.id),
            Some(current_preview)
        );
    }
}
