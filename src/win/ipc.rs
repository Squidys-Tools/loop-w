//! Same-user named-pipe command server + client.
//!
//! Ports `LoopCommandServer`/`LoopCommandClient`: pipe `LoopW-Commands`,
//! byte mode, UTF-8 no-BOM lines, 256-char / 2 s limits, 3-attempt client
//! with 250 ms connects, and the single-`ReadLine` wire quirk (multi-line
//! list replies arrive first-line-only over the pipe; the local console
//! path prints them in full).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
    SDDL_REVISION_1, SE_KERNEL_OBJECT,
};
use windows::Win32::Security::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Pipes::*;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use super::events::{push, RuntimeEvent};

pub const PIPE_NAME: &str = r"\\.\pipe\LoopW-Commands";
const MAX_COMMAND_CHARS: usize = 256;
const READ_TIMEOUT_MS: u64 = 2000;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Pending pipe replies, completed on the UI thread.
static REPLIES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u64, String>>> =
    std::sync::OnceLock::new();

fn replies() -> &'static std::sync::Mutex<std::collections::HashMap<u64, String>> {
    REPLIES.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

pub fn take_reply(id: u64) -> Option<String> {
    replies().lock().ok()?.remove(&id)
}

/// Store the UI thread's reply for a pending pipe command.
pub fn set_reply(id: u64, reply: String) {
    if let Ok(mut slots) = replies().lock() {
        slots.insert(id, reply);
    }
}

/// Spawn the pipe server thread (idempotent).
pub fn start_server() {
    static STARTED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    STARTED.get_or_init(|| {
        std::thread::Builder::new()
            .name("loopw-pipe".to_string())
            .spawn(server_loop)
            .expect("pipe thread spawns");
    });
}

static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Stop the pipe server (quit path). Unblocks the listener with a
/// self-connection, mirroring Dispose's cancellation of the accept loop.
pub fn stop_server() {
    use std::sync::atomic::Ordering;
    STOP.store(true, Ordering::SeqCst);
    // Wake ConnectNamedPipe so the loop observes STOP.
    let name = pipe_name_wide();
    unsafe {
        if WaitNamedPipeW(PCWSTR(name.as_ptr()), 500).as_bool() {
            if let Ok(pipe) = CreateFileW(
                PCWSTR(name.as_ptr()),
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) {
                let _ = CloseHandle(pipe);
            }
        }
    }
}

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(core::iter::once(0)).collect()
}

fn pipe_name_wide() -> Vec<u16> {
    wide_null(PIPE_NAME)
}

/// SECURITY_ATTRIBUTES restricting the pipe to System + current user,
/// mirroring `PipeOptions.CurrentUserOnly`.
fn current_user_sa() -> Option<(SECURITY_ATTRIBUTES, Vec<u8>)> {
    unsafe {
        // Current user SID -> string -> SDDL "D:P(A;;GA;;;SY)(A;;GA;;;sid)".
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return None;
        }
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        let mut buffer = vec![0u8; needed.max(64) as usize];
        if GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            needed,
            &mut needed,
        )
        .is_err()
        {
            let _ = CloseHandle(token);
            return None;
        }
        let _ = CloseHandle(token);
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let mut sid_string = windows::core::PWSTR::null();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid_string).is_err() {
            return None;
        }
        let sid = sid_string.to_string().unwrap_or_default();
        let _ = LocalFree(Some(HLOCAL(sid_string.0 as *mut core::ffi::c_void)));
        if sid.is_empty() {
            return None;
        }
        let sddl = format!("D:P(A;;GA;;;SY)(A;;GA;;;{sid})");
        let sddl_wide = wide_null(&sddl);
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        let mut descriptor_size = 0u32;
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl_wide.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            Some(&mut descriptor_size),
        )
        .is_err()
        {
            return None;
        }
        let sa = SECURITY_ATTRIBUTES {
            nLength: core::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: BOOL(0),
        };
        // NOTE: descriptor intentionally leaks for process lifetime; the
        // pipe server lives as long as the app and recreates pipes with it.
        Some((sa, buffer))
    }
}

fn server_loop() {
    let name = pipe_name_wide();
    // Leak the SA pair once; reused for every accepted instance.
    let sa_box: Option<Box<(SECURITY_ATTRIBUTES, Vec<u8>)>> = current_user_sa().map(Box::new);
    let sa_ptr = sa_box
        .as_ref()
        .map(|pair| &pair.0 as *const SECURITY_ATTRIBUTES);
    // Keep the box alive for the thread's lifetime.
    let _keep = sa_box;
    loop {
        if STOP.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        let pipe = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                4096,
                4096,
                0,
                sa_ptr,
            )
        };
        if pipe.is_invalid() {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        let connected = unsafe { ConnectNamedPipe(pipe, None) };
        if let Err(error) = connected {
            // ERROR_PIPE_CONNECTED: client connected between create + listen.
            if error.code() != ERROR_PIPE_CONNECTED.to_hresult() {
                unsafe {
                    let _ = CloseHandle(pipe);
                }
                continue;
            }
        }
        if let Some(command) = read_command(pipe) {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            push(RuntimeEvent::PipeCommand { id, command });
            // Wait briefly for the UI thread to execute and reply.
            let deadline = std::time::Instant::now() + Duration::from_millis(READ_TIMEOUT_MS);
            let mut replied = false;
            loop {
                if let Some(reply) = take_reply(id) {
                    write_line(pipe, &reply);
                    replied = true;
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if !replied {
                // Reap the slot so a late reply cannot leak or surprise
                // a future command reusing the id space.
                take_reply(id);
            }
        }
        unsafe {
            let _ = DisconnectNamedPipe(pipe);
            let _ = CloseHandle(pipe);
        }
    }
}

/// Read one command line (256-char cap, trailing CR trimmed).
fn read_command(pipe: HANDLE) -> Option<String> {
    let mut bytes = Vec::with_capacity(64);
    let mut chunk = [0u8; 1];
    let deadline = std::time::Instant::now() + Duration::from_millis(READ_TIMEOUT_MS);
    loop {
        if bytes.len() >= MAX_COMMAND_CHARS {
            return None;
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        let mut read = 0u32;
        if unsafe { ReadFile(pipe, Some(&mut chunk[..]), Some(&mut read), None) }.is_err() {
            // Aborted connection: drop partial bytes (C# IOException path
            // never invokes the handler). Graceful EOF (read==0 below) with
            // partial bytes still executes, matching ReadCommandAsync.
            return None;
        }
        if read == 0 {
            return if bytes.is_empty() {
                None
            } else {
                Some(decode(&bytes))
            };
        }
        if chunk[0] == b'\n' {
            return Some(decode(&bytes));
        }
        bytes.push(chunk[0]);
    }
}

fn decode(bytes: &[u8]) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    while text.ends_with('\r') {
        text.pop();
    }
    text
}

fn write_line(pipe: HANDLE, reply: &str) {
    // Single WriteLine like C#: embedded newlines pass through raw.
    let mut text = format!("{reply}\n");
    let bytes = core::mem::take(&mut text).into_bytes();
    let mut written = 0u32;
    unsafe {
        let _ = WriteFile(pipe, Some(&bytes[..]), Some(&mut written), None);
        let _ = FlushFileBuffers(pipe);
    }
}

/// Client: forward one command to the resident instance (3 attempts).
/// Returns the first reply line on success (wire quirk preserved).
pub fn try_forward_to_running(command: &str) -> Option<String> {
    let name = pipe_name_wide();
    for attempt in 0..3 {
        unsafe {
            if !WaitNamedPipeW(PCWSTR(name.as_ptr()), 250).as_bool() {
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                return None;
            }
            let open = CreateFileW(
                PCWSTR(name.as_ptr()),
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            );
            let Ok(pipe) = open else {
                // Cross-user pipes are refused immediately (C# maps
                // UnauthorizedAccessException straight to false); other
                // failures retry.
                if GetLastError() == ERROR_ACCESS_DENIED {
                    return None;
                }
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                return None;
            };
            let line = format!("{command}\n").into_bytes();
            let mut written = 0u32;
            if WriteFile(pipe, Some(&line[..]), Some(&mut written), None).is_err() {
                let _ = CloseHandle(pipe);
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                return None;
            }
            // Client-side CurrentUserOnly: refuse to talk to a pipe owned
            // by anyone but the current user.
            if !pipe_owned_by_self(pipe) {
                let _ = CloseHandle(pipe);
                return None;
            }
            let reply = read_reply_line(pipe);
            let _ = CloseHandle(pipe);
            if reply.is_some() {
                return reply;
            }
            if attempt < 2 {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    None
}

/// True when `pipe` is owned by the current user.
/// Reads the pipe's owner SID and compares it with the process token user.
fn pipe_owned_by_self(pipe: HANDLE) -> bool {
    use windows::Win32::Security::{EqualSid, OWNER_SECURITY_INFORMATION, PSID};
    unsafe {
        let mut owner: PSID = PSID::default();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        let status = GetSecurityInfo(
            pipe,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            None,
            None,
            Some(&mut descriptor),
        );
        if status != ERROR_SUCCESS {
            return false;
        }
        let owned = self_user_sid()
            .map(|mut sid| {
                EqualSid(owner, PSID(sid.as_mut_ptr() as *mut core::ffi::c_void)).is_ok()
            })
            .unwrap_or(false);
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        owned
    }
}

/// Current process user SID bytes (for owner comparison).
fn self_user_sid() -> Option<Vec<u8>> {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return None;
        }
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        let mut buffer = vec![0u8; needed.max(64) as usize];
        let capacity = buffer.len() as u32;
        let ok = GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            capacity,
            &mut needed,
        )
        .is_ok();
        let _ = CloseHandle(token);
        if !ok {
            return None;
        }
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let sid_ptr = user.User.Sid.0 as *const u8;
        if sid_ptr.is_null() {
            return None;
        }
        let length = GetLengthSid(user.User.Sid);
        if length == 0 || length > 256 {
            return None;
        }
        // Copy the SID bytes themselves (NOT the TOKEN_USER header: the
        // struct starts with the SID pointer + attributes).
        Some(core::slice::from_raw_parts(sid_ptr, length as usize).to_vec())
    }
}

/// Single ReadLine (first line only) with a 2 s deadline.
fn read_reply_line(pipe: HANDLE) -> Option<String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 1];
    let deadline = std::time::Instant::now() + Duration::from_millis(READ_TIMEOUT_MS);
    loop {
        if std::time::Instant::now() >= deadline {
            return None;
        }
        let mut read = 0u32;
        if unsafe { ReadFile(pipe, Some(&mut chunk[..]), Some(&mut read), None) }.is_err() {
            return None;
        }
        if read == 0 {
            // Clean EOF: C# ReadLineAsync yields null -> success with "".
            return Some(decode(&bytes));
        }
        if chunk[0] == b'\n' {
            return Some(decode(&bytes));
        }
        bytes.push(chunk[0]);
    }
}

/// Reply for one incoming command (pure; also used by tests).
pub fn reply_for(command: &str) -> String {
    match crate::core::commands::parse_command(command) {
        Ok(_) => "OK".to_string(),
        Err(error) => format!("ERROR: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same-user pipe ownership roundtrip: a pipe we create must verify
    /// as self-owned (guards the CurrentUserOnly client check).
    #[test]
    fn pipe_owner_verifies_self_owned_pipe() {
        use windows::Win32::Security::PSID;
        let sid = self_user_sid();
        assert!(sid.is_some(), "self_user_sid is None");
        // Roundtrip against a freshly created pipe owned by us.
        let name = format!(
            r"\\.\pipe\LoopW-Debug-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let wide = wide_null(&name);
        unsafe {
            let server = CreateNamedPipeW(
                PCWSTR(wide.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                4096,
                4096,
                0,
                None,
            );
            assert!(!server.is_invalid(), "create pipe");
            let client = CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
            .expect("open pipe");
            let owned = pipe_owned_by_self(client);
            let _ = CloseHandle(client);
            let _ = CloseHandle(server);
            assert!(owned, "owner check fails on own pipe");
        }
        let _ = PSID::default();
    }
}
