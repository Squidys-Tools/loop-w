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

pub(super) fn tray_menu_settings(position: crate::core::rect::Point) -> window::Settings {
    let scale = crate::win::native::dpi_scale_at_point(position).max(0.01);
    let monitor = crate::win::monitor_service::for_point(position)
        .map(|snapshot| snapshot.work)
        .unwrap_or_else(|| {
            crate::core::rect::Rect::new(
                position.x - 224,
                position.y - 156,
                position.x + 224,
                position.y + 156,
            )
        });
    let frame = tray_menu_frame(position, monitor, scale);
    window::Settings {
        size: iced::Size::new(
            frame.width() as f32 / scale as f32,
            frame.height() as f32 / scale as f32,
        ),
        position: window::Position::Specific(iced::Point::new(
            frame.left as f32 / scale as f32,
            frame.top as f32 / scale as f32,
        )),
        resizable: false,
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        exit_on_close_request: false,
        icon: Some(crate::win::tray::window_icon()),
        ..Default::default()
    }
}

fn tray_menu_frame(
    position: crate::core::rect::Point,
    monitor: crate::core::rect::Rect,
    scale: f64,
) -> crate::core::rect::Rect {
    const WIDTH: i32 = 224;
    const HEIGHT: i32 = 156;
    let physical_width = (WIDTH as f64 * scale).round() as i32;
    let physical_height = (HEIGHT as f64 * scale).round() as i32;
    let mut left = position.x - physical_width;
    let mut top = position.y - physical_height;
    if top < monitor.top {
        top = position.y;
    }
    if left < monitor.left {
        left = position.x;
    }
    left = left.clamp(
        monitor.left,
        (monitor.right - physical_width).max(monitor.left),
    );
    top = top.clamp(
        monitor.top,
        (monitor.bottom - physical_height).max(monitor.top),
    );
    crate::core::rect::Rect::new(left, top, left + physical_width, top + physical_height)
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

#[cfg(test)]
mod tests {
    use super::tray_menu_frame;
    use crate::core::rect::{Point, Rect};

    #[test]
    fn tray_menu_frame_anchors_above_right_edge() {
        let frame = tray_menu_frame(Point::new(1920, 1080), Rect::new(0, 0, 1920, 1080), 1.0);

        assert_eq!(frame, Rect::new(1696, 924, 1920, 1080));
    }

    #[test]
    fn tray_menu_frame_flips_inside_top_left_monitor_edges() {
        let frame = tray_menu_frame(Point::new(0, 0), Rect::new(0, 0, 1920, 1080), 1.0);

        assert_eq!(frame, Rect::new(0, 0, 224, 156));
    }

    #[test]
    fn tray_menu_frame_scales_physical_size() {
        let frame = tray_menu_frame(Point::new(1920, 1080), Rect::new(0, 0, 2560, 1440), 1.5);

        assert_eq!(frame.width(), 336);
        assert_eq!(frame.height(), 234);
        assert_eq!(frame.right, 1920);
        assert_eq!(frame.bottom, 1080);
    }
}
