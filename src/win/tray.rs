//! System-tray resident icon: tooltip, menu, click behavior.
//!
//! Ports `TrayIcon`: `Open LoopW` / `Open settings` / separator / `Quit`,
//! double-click shows settings, icon generated programmatically (blue ring
//! on dark) so no binary assets are needed. Events are polled from the UI
//! frame tick and translated to [`RuntimeEvent`]s.

use super::events::{push, RuntimeEvent};
use crate::core::rect::Point;

pub struct Tray {
    _icon: tray_icon::TrayIcon,
}

/// Actions surfaced to the iced runtime (kept for compat).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ShowSettings,
    Quit,
}

/// Build the resident tray icon + menu. Must be called on the main thread.
pub fn build() -> Result<Tray, String> {
    let icon = tray_icon::TrayIconBuilder::new()
        .with_menu_on_left_click(false)
        .with_tooltip("LoopW")
        .with_title("LoopW")
        .with_icon(loop_icon())
        .build()
        .map_err(|error| format!("Could not create the tray icon: {error}"))?;
    Ok(Tray { _icon: icon })
}

/// Poll tray + menu events (call once per frame). Never blocks.
pub fn poll(_tray: &Tray) {
    use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        match event {
            TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                push(RuntimeEvent::TrayShowSettings);
            }
            TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                position,
                ..
            } => {
                push(RuntimeEvent::TrayMenuRequested {
                    position: Point::new(position.x.round() as i32, position.y.round() as i32),
                });
            }
            _ => {}
        }
    }
}

/// Blue-ring LoopW mark rendered into RGBA (64x64).
fn loop_icon_rgba() -> Vec<u8> {
    loop_icon_rgba_sized(64)
}

fn loop_icon_rgba_sized(size: u32) -> Vec<u8> {
    let size = size as usize;
    let mut rgba = vec![0u8; size * size * 4];
    let center = size as f64 / 2.0;
    let scale = size as f64 / 64.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 + 0.5 - center;
            let dy = y as f64 + 0.5 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let (r, g, b, a) = if (distance - 22.0 * scale).abs() <= 4.0 * scale {
                (61u8, 155, 255, 255)
            } else if distance < 18.0 * scale {
                (27, 33, 43, 255)
            } else {
                (0, 0, 0, 0)
            };
            let offset = (y * size + x) * 4;
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
            rgba[offset + 3] = a;
        }
    }
    rgba
}

fn loop_icon() -> tray_icon::Icon {
    tray_icon::Icon::from_rgba(loop_icon_rgba(), 64, 64).expect("generated icon is valid RGBA")
}

pub fn window_icon() -> iced::window::Icon {
    iced::window::icon::from_rgba(loop_icon_rgba_sized(32), 32, 32)
        .expect("generated icon is valid RGBA")
}
