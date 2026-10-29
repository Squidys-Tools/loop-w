//! Pipe server: one accept loop on a dedicated thread. Each command line
//! becomes a `PipeCommand` runtime event; the UI thread's reply is written
//! back as a single line.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Pipes::*;

use super::replies::wait_reply;
use super::security::current_user_sa;
use super::wire::{decode, pipe_name_wide, MAX_COMMAND_CHARS, READ_TIMEOUT_MS};
use crate::win::events::{push, RuntimeEvent};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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

fn server_loop() {
    let name = pipe_name_wide();
    // Keep the descriptor and SID storage alive while accepted pipes reuse
    // the same SECURITY_ATTRIBUTES pair. The descriptor is freed when the
    // server thread exits.
    let sa_box = current_user_sa().map(Box::new);
    let sa_ptr = sa_box
        .as_ref()
        .map(|pair| &pair.attributes as *const SECURITY_ATTRIBUTES);
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
            crate::win::diagnostics::report_ipc(
                "command pipe unavailable",
                "CreateNamedPipeW failed; retrying",
            );
            std::thread::sleep(Duration::from_millis(500));
            continue;
        }
        let connected = unsafe { ConnectNamedPipe(pipe, None) };
        if let Err(error) = connected {
            // ERROR_PIPE_CONNECTED: client connected between create + listen.
            if error.code() != ERROR_PIPE_CONNECTED.to_hresult() {
                crate::win::diagnostics::report_ipc(
                    "client connect failed",
                    "ConnectNamedPipe returned an unexpected error",
                );
                unsafe {
                    let _ = CloseHandle(pipe);
                }
                continue;
            }
        }
        if let Some(command) = read_command(pipe) {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            push(RuntimeEvent::PipeCommand { id, command });
            // Sleep until the UI thread publishes the reply instead of
            // polling the shared map every 10 ms.
            if let Some(reply) = wait_reply(id) {
                write_line(pipe, &reply);
            } else {
                crate::win::diagnostics::report_ipc(
                    "command reply timed out",
                    "UI thread did not answer within 2 s",
                );
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
