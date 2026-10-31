//! Pipe client: forwards one command line to the resident and reads the
//! single reply line. Retries only until the command bytes are written —
//! a lost reply after a successful write is never resent, so a
//! non-idempotent command cannot run twice.

use std::time::Duration;

use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Pipes::WaitNamedPipeW;

use super::security::pipe_owned_by_self;
use super::wire::{decode, pipe_name_wide, READ_TIMEOUT_MS};

/// Result of one forward attempt. Distinguishes "never on the wire"
/// (caller may try another path) from "on the wire, reply lost" (caller
/// must never send the same non-idempotent command again).
#[derive(Debug, PartialEq, Eq)]
pub enum ForwardOutcome {
    /// Resident answered with a reply line (may be empty on clean EOF).
    Replied(String),
    /// Command bytes never left this process (no pipe / open / write fail).
    NotDelivered,
    /// Bytes were written; the reply was lost or timed out. Never resend.
    DeliveredNoReply,
}

impl ForwardOutcome {
    pub fn reply(&self) -> Option<&str> {
        match self {
            ForwardOutcome::Replied(text) => Some(text),
            _ => None,
        }
    }

    pub fn is_delivered(&self) -> bool {
        matches!(
            self,
            ForwardOutcome::Replied(_) | ForwardOutcome::DeliveredNoReply
        )
    }
}

/// Client: forward one command to the resident instance.
/// Retries only until the command bytes are written; a lost reply after a
/// successful write is [`ForwardOutcome::DeliveredNoReply`] and never
/// resent (see [`try_forward_attempts`]).
pub fn try_forward_to_running(command: &str) -> ForwardOutcome {
    try_forward_attempts(command, 3)
}

/// Patient forward for the startup race: the resident may have just won the
/// instance mutex and not yet bound the pipe. ~6s of WaitNamedPipe windows
/// with short sleeps between them. Same no-resend-after-write rule as
/// [`try_forward_to_running`]: non-idempotent commands (nudge, cycle) must
/// not run twice if the reply is delayed past the read deadline.
pub fn try_forward_patiently(command: &str) -> ForwardOutcome {
    try_forward_attempts(command, 24)
}

fn try_forward_attempts(command: &str, attempts: u32) -> ForwardOutcome {
    let attempts = attempts.max(1);
    let name = pipe_name_wide();
    for attempt in 0..attempts {
        unsafe {
            if !WaitNamedPipeW(PCWSTR(name.as_ptr()), 250).as_bool() {
                if attempt + 1 < attempts {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                return ForwardOutcome::NotDelivered;
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
                    return ForwardOutcome::NotDelivered;
                }
                if attempt + 1 < attempts {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                return ForwardOutcome::NotDelivered;
            };
            let line = format!("{command}\n").into_bytes();
            let mut written = 0u32;
            if WriteFile(pipe, Some(&line[..]), Some(&mut written), None).is_err() {
                let _ = CloseHandle(pipe);
                if attempt + 1 < attempts {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                return ForwardOutcome::NotDelivered;
            }
            // Client-side CurrentUserOnly: refuse to talk to a pipe owned
            // by anyone but the current user. Bytes may already be written;
            // report NotDelivered so callers can fall back without treating
            // this as a successful hand-off to *our* resident.
            if !pipe_owned_by_self(pipe) {
                let _ = CloseHandle(pipe);
                return ForwardOutcome::NotDelivered;
            }
            // Bytes are on the wire. Never resend: a delayed or lost reply
            // must not re-run a non-idempotent command on the resident.
            let reply = read_reply_line(pipe);
            let _ = CloseHandle(pipe);
            return match reply {
                Some(text) => ForwardOutcome::Replied(text),
                None => ForwardOutcome::DeliveredNoReply,
            };
        }
    }
    ForwardOutcome::NotDelivered
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Callers must not treat a lost reply as "safe to retry another path."
    #[test]
    fn forward_outcome_separates_delivery_from_reply_loss() {
        assert_eq!(ForwardOutcome::Replied("OK".into()).reply(), Some("OK"));
        assert_eq!(ForwardOutcome::DeliveredNoReply.reply(), None);
        assert_eq!(ForwardOutcome::NotDelivered.reply(), None);
        assert!(ForwardOutcome::Replied(String::new()).is_delivered());
        assert!(ForwardOutcome::DeliveredNoReply.is_delivered());
        assert!(!ForwardOutcome::NotDelivered.is_delivered());
    }
}
