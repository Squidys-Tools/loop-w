//! Safe wrappers over the `windows` crate for every Win32 call LoopW needs.
//!
//! All functions are small, total-failure-safe (bool/Option/empty defaults),
//! and physical-pixel based. UI code never touches raw Win32 directly.

use std::cell::{Cell, RefCell};
use std::sync::{Mutex, OnceLock};

use windows::core::{BOOL, GUID, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, GetDoubleClickTime};
use windows::Win32::UI::Shell::ITaskbarList;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::identity::WindowIdentity;
use crate::core::rect::{Point, Rect};

/// Raw window handle as an integer (HWND.0), usable as a map key.
pub type RawHwnd = isize;

/// Winit's always-alive message target. It is visible by design so that
/// Windows delivers paint messages, but it must never get a taskbar button.
pub const WINIT_EVENT_TARGET_CLASS: &str = "Winit Thread Event Target";

thread_local! {
    static TASKBAR_COM_READY: Cell<Option<bool>> = const { Cell::new(None) };
    static TASKBAR_LIST: RefCell<Option<ITaskbarList>> = const { RefCell::new(None) };
}

static EVENT_TARGET_CACHE: OnceLock<Mutex<Option<RawHwnd>>> = OnceLock::new();
static SUPPRESSED_TASKBAR_CACHE: OnceLock<Mutex<Option<RawHwnd>>> = OnceLock::new();

fn event_target_cache() -> &'static Mutex<Option<RawHwnd>> {
    EVENT_TARGET_CACHE.get_or_init(|| Mutex::new(None))
}

fn suppressed_taskbar_cache() -> &'static Mutex<Option<RawHwnd>> {
    SUPPRESSED_TASKBAR_CACHE.get_or_init(|| Mutex::new(None))
}

pub fn raw(hwnd: HWND) -> RawHwnd {
    hwnd.0 as RawHwnd
}

// Bug-compat note: HWND.0 is *mut c_void; cast through usize is fine.
pub fn from_raw(value: RawHwnd) -> HWND {
    HWND(value as *mut core::ffi::c_void)
}

pub fn rect_from_native(r: RECT) -> Rect {
    Rect::new(r.left, r.top, r.right, r.bottom)
}

pub fn rect_to_native(r: Rect) -> RECT {
    RECT {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}

pub fn is_window(hwnd: HWND) -> bool {
    unsafe { IsWindow(Some(hwnd)).as_bool() }
}

pub fn is_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() }
}

pub fn is_iconic(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).as_bool() }
}

pub fn is_zoomed(hwnd: HWND) -> bool {
    unsafe { IsZoomed(hwnd).as_bool() }
}

pub fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn window_rect(hwnd: HWND) -> Option<Rect> {
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;
        Some(rect_from_native(rect))
    }
}

pub fn window_placement(hwnd: HWND) -> Option<WINDOWPLACEMENT> {
    unsafe {
        let mut placement = WINDOWPLACEMENT {
            length: core::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        GetWindowPlacement(hwnd, &mut placement).ok()?;
        Some(placement)
    }
}

pub fn set_placement(hwnd: HWND, placement: &WINDOWPLACEMENT) -> bool {
    unsafe { SetWindowPlacement(hwnd, placement).is_ok() }
}

pub fn set_pos(hwnd: HWND, frame: Rect) -> bool {
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            frame.left,
            frame.top,
            frame.width(),
            frame.height(),
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
        )
        .is_ok()
    }
}

pub fn show_window(hwnd: HWND, cmd: SHOW_WINDOW_CMD) -> bool {
    unsafe { ShowWindow(hwnd, cmd).as_bool() }
}

pub fn set_foreground(hwnd: HWND) -> bool {
    unsafe { SetForegroundWindow(hwnd).as_bool() }
}

pub fn process_id(hwnd: HWND) -> u32 {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid
    }
}

pub fn window_style(hwnd: HWND) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) }
}

pub fn window_ex_style(hwnd: HWND) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) }
}

pub fn window_owner(hwnd: HWND) -> HWND {
    unsafe { GetWindow(hwnd, GW_OWNER).unwrap_or_default() }
}

pub fn window_class(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = [0u16; 256];
        let length = GetClassNameW(hwnd, &mut buffer);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
}

pub fn window_title(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = [0u16; 512];
        let length = GetWindowTextW(hwnd, &mut buffer);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
}

/// Executable path via OpenProcess + QueryFullProcessImageName.
/// Empty when denied (elevated/protected) — tolerated, never an error.
pub fn executable_path(pid: u32) -> String {
    use windows::Win32::System::Threading::*;
    if pid == 0 {
        return String::new();
    }
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let path = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .map(|_| String::from_utf16_lossy(&buffer[..size as usize]))
        .unwrap_or_default();
        let _ = CloseHandle(handle);
        path
    }
}

/// Process basename without extension (`C:\A\app.exe` -> `app`).
pub fn process_name(pid: u32) -> String {
    use windows::Win32::System::Threading::*;
    if pid == 0 {
        return String::new();
    }
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        let name = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .map(|_| {
            let full = String::from_utf16_lossy(&buffer[..size as usize]);
            let file = full.rsplit(['/', '\\']).next().unwrap_or(&full);
            // Strip only the extension (my.app.exe -> my.app), like C#.
            file.rsplit_once('.')
                .map(|(stem, _)| stem)
                .unwrap_or(file)
                .to_string()
        })
        .unwrap_or_default();
        let _ = CloseHandle(handle);
        name
    }
}

/// Full identity read for stash / policy. Fails only on invalid HWND,
/// zero PID, or unreadable class; title/exe failures are tolerated.
pub fn window_identity(hwnd: HWND) -> Option<WindowIdentity> {
    if !is_window(hwnd) {
        return None;
    }
    let pid = process_id(hwnd);
    if pid == 0 {
        return None;
    }
    let class = window_class(hwnd);
    if class.is_empty() {
        return None;
    }
    Some(WindowIdentity {
        executable_path: executable_path(pid),
        process_id: pid,
        window_class: class,
        title: window_title(hwnd),
    })
}

/// WM_GETMINMAXINFO via SendMessageTimeout (200 ms, never blocks on hung apps).
pub fn min_max_info(hwnd: HWND) -> MINMAXINFO {
    unsafe {
        let mut info = MINMAXINFO::default();
        let mut result = 0usize;
        let sent = SendMessageTimeoutW(
            hwnd,
            WM_GETMINMAXINFO,
            WPARAM(0),
            LPARAM(&mut info as *mut MINMAXINFO as isize),
            SMTO_ABORTIFHUNG,
            200,
            Some(&mut result),
        );
        if sent.0 == 0 {
            return MINMAXINFO::default();
        }
        info
    }
}

/// NCHITTEST at a point for drag-start caption detection (100 ms timeout).
pub fn nc_hit_test(hwnd: HWND, point: Point) -> Option<usize> {
    unsafe {
        let packed = (point.x as u32 as isize) | ((point.y as u32 as isize) << 32);
        let mut result = 0usize;
        let sent = SendMessageTimeoutW(
            hwnd,
            WM_NCHITTEST,
            WPARAM(0),
            LPARAM(packed),
            SMTO_ABORTIFHUNG,
            100,
            Some(&mut result),
        );
        if sent.0 == 0 {
            return None;
        }
        Some(result)
    }
}

pub fn window_from_point(point: Point) -> HWND {
    unsafe {
        let native = POINT {
            x: point.x,
            y: point.y,
        };
        WindowFromPoint(native)
    }
}

pub fn ancestor_root(hwnd: HWND) -> HWND {
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOT);
        if root.is_invalid() {
            hwnd
        } else {
            root
        }
    }
}

pub fn cursor_pos() -> Option<Point> {
    unsafe {
        let mut point = POINT::default();
        GetCursorPos(&mut point).ok()?;
        Some(Point::new(point.x, point.y))
    }
}

pub fn async_key_down(vk: i32) -> bool {
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

pub fn tick_count() -> u64 {
    unsafe { GetTickCount64() }
}

pub fn double_click_time() -> u32 {
    unsafe { GetDoubleClickTime() }
}

pub fn dpi_for_window(hwnd: HWND) -> u32 {
    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        if dpi > 0 {
            return dpi;
        }
        // Fallback: owning monitor's effective DPI (pre-PerMonitorV2 hosts).
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        if !monitor.is_invalid() {
            let mut dpi_x = 0u32;
            let mut dpi_y = 0u32;
            if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok()
                && dpi_x > 0
            {
                return dpi_x;
            }
        }
        96
    }
}

pub fn dpi_scale_for_window(hwnd: HWND) -> f64 {
    let dpi = dpi_for_window(hwnd);
    if dpi > 0 {
        dpi as f64 / 96.0
    } else {
        1.0
    }
}

/// DPI scale at a physical screen point (for overlay placement).
pub fn dpi_scale_at_point(point: Point) -> f64 {
    super::monitor_service::for_point(point)
        .map(|snapshot| {
            let dpi = snapshot.dpi_x;
            if dpi.is_finite() && dpi > 0.0 {
                dpi / 96.0
            } else {
                1.0
            }
        })
        .unwrap_or(1.0)
}

/// Make an overlay window click-through + tool (no taskbar/Alt+Tab).
pub fn make_overlay_click_through(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            style | WS_EX_TRANSPARENT.0 as isize | WS_EX_TOOLWINDOW.0 as isize,
        );
    }
}

pub fn find_window_by_title(title: &str) -> Option<HWND> {
    let wide: Vec<u16> = title.encode_utf16().chain(core::iter::once(0)).collect();
    unsafe {
        FindWindowW(PCWSTR::null(), PCWSTR(wide.as_ptr()))
            .ok()
            .filter(|hwnd| !hwnd.is_invalid())
    }
}

pub fn find_window_by_class(class: &str) -> Option<HWND> {
    if class == WINIT_EVENT_TARGET_CLASS {
        if let Ok(mut cached) = event_target_cache().lock() {
            if let Some(raw_hwnd) = *cached {
                let hwnd = from_raw(raw_hwnd);
                if is_window(hwnd) && process_id(hwnd) == own_process_id() {
                    return Some(hwnd);
                }
                *cached = None;
            }
        }
    }

    struct Search {
        pid: u32,
        class: String,
        found: Option<HWND>,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        if process_id(hwnd) != search.pid {
            return TRUE;
        }
        let current = window_class(hwnd);
        if current == search.class {
            search.found = Some(hwnd);
            return FALSE;
        }
        TRUE
    }

    let mut search = Search {
        pid: own_process_id(),
        class: class.to_string(),
        found: None,
    };
    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(&mut search as *mut Search as isize));
    }
    if class == WINIT_EVENT_TARGET_CLASS {
        if let Some(hwnd) = search.found {
            if let Ok(mut cached) = event_target_cache().lock() {
                *cached = Some(raw(hwnd));
            }
            return Some(hwnd);
        }
    }
    search.found
}

/// Remove a helper window from the shell taskbar without hiding it from
/// Winit. Winit keeps its event-target window visible because that is needed
/// for WM_PAINT delivery; changing its extended style and deleting any stale
/// shell tab is the safe app-level correction.
pub fn suppress_taskbar_window(hwnd: HWND) -> bool {
    if !is_window(hwnd) || process_id(hwnd) != own_process_id() {
        return false;
    }

    let current = window_ex_style(hwnd);
    let desired = (current & !(WS_EX_APPWINDOW.0 as isize)) | WS_EX_TOOLWINDOW.0 as isize;
    if desired != current {
        unsafe {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }

    if desired == current
        && suppressed_taskbar_cache()
            .lock()
            .map(|cached| *cached == Some(raw(hwnd)))
            .unwrap_or(false)
    {
        return true;
    }

    let deleted = delete_taskbar_tab(hwnd);
    if deleted {
        if let Ok(mut cached) = suppressed_taskbar_cache().lock() {
            *cached = Some(raw(hwnd));
        }
    }
    deleted
}

fn delete_taskbar_tab(hwnd: HWND) -> bool {
    TASKBAR_COM_READY.with(|ready| {
        if ready.get().is_none() {
            let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
            ready.set(Some(initialized));
        }
        if ready.get() != Some(true) {
            return false;
        }

        TASKBAR_LIST.with(|slot| {
            if slot.borrow().is_none() {
                let clsid = GUID::from_values(
                    0x56fdf344,
                    0xfd6d,
                    0x11d0,
                    [0x95, 0x8a, 0x00, 0x60, 0x97, 0xc9, 0xa0, 0x90],
                );
                let Ok(taskbar) = (unsafe {
                    CoCreateInstance::<_, ITaskbarList>(&clsid, None, CLSCTX_INPROC_SERVER)
                }) else {
                    return false;
                };
                if unsafe { taskbar.HrInit() }.is_err() {
                    return false;
                }
                slot.borrow_mut().replace(taskbar);
            }
            slot.borrow()
                .as_ref()
                .is_some_and(|taskbar| unsafe { taskbar.DeleteTab(hwnd).is_ok() })
        })
    })
}

pub fn own_process_id() -> u32 {
    std::process::id()
}

/// Print one CLI reply line on the parent console (GUI subsystem has none).
/// When stdout is already redirected (pipe/file capture), write directly —
/// no console API involved. Otherwise attach to the parent console like C#
/// `WriteCliResponse`; silent when there is no parent console.
/// NOTE: PowerShell `$()`/file redirection does not propagate std handles
/// into GUI-subsystem processes, so captured output only appears in hosts
/// that set up inheritance (cmd.exe console output works); this matches C#
/// byte-for-byte since the Win32 mechanics are identical.
pub fn print_cli_line(reply: &str) {
    use windows::Win32::Storage::FileSystem::{GetFileType, FILE_TYPE_DISK, FILE_TYPE_PIPE};
    use windows::Win32::System::Console::*;
    unsafe {
        if let Ok(stdout) = GetStdHandle(STD_OUTPUT_HANDLE) {
            if !stdout.is_invalid() {
                let file_type = GetFileType(stdout);
                if file_type == FILE_TYPE_PIPE || file_type == FILE_TYPE_DISK {
                    println!("{reply}");
                    return;
                }
            }
        }
        if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
            return;
        }
        println!("{reply}");
        let _ = FreeConsole();
    }
}
