use iced::window;
use iced::Task;

use super::app::{Message, State};
use crate::win;

pub(super) fn main_settings() -> window::Settings {
    window::Settings {
        size: iced::Size::new(1166.0, 779.0),
        min_size: Some(iced::Size::new(1100.0, 760.0)),
        position: window::Position::Centered,
        resizable: true,
        decorations: true,
        transparent: false,
        level: window::Level::Normal,
        exit_on_close_request: false,
        icon: Some(win::tray::window_icon()),
        ..Default::default()
    }
}

pub(super) fn radial_settings(x: f32, y: f32, w: f32, h: f32) -> window::Settings {
    overlay_settings(x, y, w, h)
}

pub(super) fn preview_settings(x: f32, y: f32, w: f32, h: f32) -> window::Settings {
    overlay_settings(x, y, w, h)
}

fn overlay_settings(x: f32, y: f32, w: f32, h: f32) -> window::Settings {
    window::Settings {
        size: iced::Size::new(w, h),
        position: window::Position::Specific(iced::Point::new(x, y)),
        resizable: false,
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        ..Default::default()
    }
}

pub(super) fn show_main(state: &mut State, tasks: &mut Vec<Task<Message>>) {
    match state.main_id {
        Some(id) => tasks.push(window::gain_focus(id)),
        None => {
            let (id, task) = window::open(main_settings());
            state.main_id = Some(id);
            tasks.push(task.map(Message::MainOpened));
        }
    }
}
