//! Cached monitor enumeration + DPI-aware work areas.
//!
//! Ports `MonitorService`: generation counter, HMONITOR-keyed snapshot
//! cache, sorted monitor list, and padding/translation math from
//! `crate::core::monitor`. Invalidated on display/DPI/setting changes.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use windows::core::BOOL;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::HiDpi::*;

use super::native;
use crate::core::monitor::{apply_padding, MonitorMoveSizePolicy, MonitorSnapshot};

struct Cache {
    generation: u64,
    snapshots: HashMap<isize, MonitorSnapshot>,
    all: Option<Vec<MonitorSnapshot>>,
}

static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

fn cache() -> &'static Mutex<Cache> {
    CACHE.get_or_init(|| {
        Mutex::new(Cache {
            generation: 0,
            snapshots: HashMap::new(),
            all: None,
        })
    })
}

pub fn generation() -> u64 {
    cache().lock().map(|c| c.generation).unwrap_or(0)
}

pub fn move_size_policy() -> MonitorMoveSizePolicy {
    super::shared::snapshot().monitor_move_policy
}

/// Drop all cached snapshots (display/DPI/settings changed).
pub fn invalidate() {
    if let Ok(mut guard) = cache().lock() {
        guard.generation += 1;
        guard.snapshots.clear();
        guard.all = None;
    }
}

/// Snapshot for the monitor containing a window (raw HWND as u64).
pub fn for_window(hwnd: u64) -> Option<MonitorSnapshot> {
    let monitor =
        unsafe { MonitorFromWindow(native::from_raw(hwnd as isize), MONITOR_DEFAULTTONEAREST) };
    read(monitor)
}

/// Snapshot for the monitor containing a rect.
pub fn for_rect(rect: crate::core::rect::Rect) -> Option<MonitorSnapshot> {
    let native_rect = native::rect_to_native(rect);
    let monitor = unsafe { MonitorFromRect(&native_rect as *const RECT, MONITOR_DEFAULTTONEAREST) };
    read(monitor)
}

/// Snapshot for the monitor containing a point.
pub fn for_point(point: crate::core::rect::Point) -> Option<MonitorSnapshot> {
    let monitor = unsafe {
        MonitorFromPoint(
            windows::Win32::Foundation::POINT {
                x: point.x,
                y: point.y,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };
    read(monitor)
}

/// All monitors sorted by work-area left, then top.
pub fn all() -> Vec<MonitorSnapshot> {
    if let Ok(guard) = cache().lock() {
        if let Some(all) = guard.all.clone() {
            return all;
        }
    }
    let mut monitors: Vec<MonitorSnapshot> = Vec::new();
    unsafe extern "system" fn enum_proc(
        monitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        _data: LPARAM,
    ) -> BOOL {
        if let Some(snapshot) = read(monitor) {
            let out = &mut *(_data.0 as *mut Vec<MonitorSnapshot>);
            out.push(snapshot);
        }
        BOOL::from(true)
    }
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_proc),
            LPARAM(&mut monitors as *mut Vec<MonitorSnapshot> as isize),
        );
    }
    monitors.sort_by_key(|snapshot| (snapshot.work.left, snapshot.work.top));
    if let Ok(mut guard) = cache().lock() {
        guard.all = Some(monitors.clone());
    }
    monitors
}

fn read(monitor: HMONITOR) -> Option<MonitorSnapshot> {
    if monitor.is_invalid() {
        super::diagnostics::report_monitor("no monitor handle for this window or rect");
        return None;
    }
    let key = monitor.0 as isize;
    if let Ok(guard) = cache().lock() {
        if let Some(snapshot) = guard.snapshots.get(&key) {
            return Some(*snapshot);
        }
    }
    let mut info = MONITORINFO {
        cbSize: core::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let (dpi_x, dpi_y) = monitor_dpi(monitor);
            let settings = super::shared::snapshot();
            let work = apply_padding(
                native::rect_from_native(info.rcWork),
                settings.global_padding,
                settings.padding_left,
                settings.padding_top,
                settings.padding_right,
                settings.padding_bottom,
            );
            let snapshot = MonitorSnapshot {
                monitor: native::rect_from_native(info.rcMonitor),
                work,
                dpi_x: normalize_dpi(dpi_x),
                dpi_y: normalize_dpi(dpi_y),
            };
            if let Ok(mut guard) = cache().lock() {
                guard.snapshots.insert(key, snapshot);
            }
            return Some(snapshot);
        }
    }
    super::diagnostics::report_monitor("the system refused monitor details for a live handle");
    None
}

fn monitor_dpi(monitor: HMONITOR) -> (f64, f64) {
    unsafe {
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok() {
            return (dpi_x as f64, dpi_y as f64);
        }
    }
    (96.0, 96.0)
}

fn normalize_dpi(dpi: f64) -> f64 {
    if dpi.is_finite() && dpi > 0.0 {
        dpi
    } else {
        96.0
    }
}
