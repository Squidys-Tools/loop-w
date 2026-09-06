//! System-tray resident icon: tooltip, menu, click behavior.
//!
//! Ports `TrayIcon`: `Open LoopW` / `Open settings` / separator / `Quit`,
//! double-click shows settings, icon generated programmatically (blue ring
//! on dark) so no binary assets are needed. Events are polled from the UI
//! frame tick and translated to [`RuntimeEvent`]s.

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};

use super::events::{RuntimeEvent, push};

pub struct Tray {
    _icon: tray_icon::TrayIcon,
    open: MenuId,
    settings: MenuId,
    quit: MenuId,
}

/// Actions surfaced to the iced runtime (kept for compat).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ShowSettings,
    Quit,
}

/// Build the resident tray icon + menu. Must be called on the main thread.
pub fn build() -> Result<Tray, String> {
    let menu = Menu::new();
    let open = MenuItem::new("Open LoopW", true, None);
    let settings = MenuItem::new("Open settings", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append_items(&[
        &open,
        &settings,
        &PredefinedMenuItem::separator(),
        &quit,
    ])
    .map_err(|error| format!("Could not build the tray menu: {error}"))?;
    let icon = tray_icon::TrayIconBuilder::new()
        .with_menu_on_left_click(false)
        .with_tooltip("LoopW")
        .with_title("LoopW")
        .with_menu(Box::new(menu))
        .with_icon(loop_icon())
        .build()
        .map_err(|error| format!("Could not create the tray icon: {error}"))?;
    Ok(Tray {
        _icon: icon,
        open: open.id().clone(),
        settings: settings.id().clone(),
        quit: quit.id().clone(),
    })
}

/// Poll tray + menu events (call once per frame). Never blocks.
pub fn poll(tray: &Tray) {
    use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        match event {
            TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => {
                push(RuntimeEvent::TrayShowSettings);
            }
            TrayIconEvent::Click { .. } => {
                let _ = MouseButtonState::Up;
            }
            _ => {}
        }
    }
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        if event.id == tray.open || event.id == tray.settings {
            push(RuntimeEvent::TrayShowSettings);
        } else if event.id == tray.quit {
            push(RuntimeEvent::TrayQuit);
        }
    }
}

/// Blue-ring LoopW mark rendered into RGBA (64x64).
fn loop_icon() -> tray_icon::Icon {
    const SIZE: usize = 64;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let center = SIZE as f64 / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f64 + 0.5 - center;
            let dy = y as f64 + 0.5 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let (r, g, b, a) = if (distance - 22.0).abs() <= 4.0 {
                (61u8, 155, 255, 255)
            } else if distance < 18.0 {
                (27, 33, 43, 255)
            } else {
                (0, 0, 0, 0)
            };
            let offset = (y * SIZE + x) * 4;
            rgba[offset] = r;
            rgba[offset + 1] = g;
            rgba[offset + 2] = b;
            rgba[offset + 3] = a;
        }
    }
    tray_icon::Icon::from_rgba(rgba, SIZE as u32, SIZE as u32)
        .expect("generated icon is valid RGBA")
}
