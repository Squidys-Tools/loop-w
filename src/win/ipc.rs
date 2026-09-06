//! Same-user named-pipe command server (`LoopW.exe <command>`).
//!
//! Ports `LoopCommandServer`: a second process forwards its single command
//! arg to the resident instance instead of starting a second hook set.

/// Pipe name used by both the C# and Rust builds for compat.
pub const PIPE_NAME: &str = r"\\.\pipe\LoopW-Command";

/// Try to forward `command` to a running instance. Returns true if sent.
pub fn try_forward_to_running(_command: &str) -> bool {
    // Live pipe client lands here (Windows-only). Returns false so the
    // first instance handles the command locally until wired.
    false
}

/// Reply for one incoming pipe command (pure, tested indirectly via core).
pub fn reply_for(command: &str) -> String {
    match crate::core::commands::parse_command(command) {
        Ok(_) => "OK".to_string(),
        Err(error) => format!("ERROR: {error}"),
    }
}
