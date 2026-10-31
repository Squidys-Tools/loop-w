//! Same-user named-pipe command server + client.
//!
//! Ports `LoopCommandServer`/`LoopCommandClient`: pipe `LoopW-Commands`,
//! byte mode, UTF-8 no-BOM lines, 256-char / 2 s limits, a client that
//! retries only until the command is written (never after), and the
//! single-`ReadLine` wire quirk (multi-line list replies arrive
//! first-line-only over the pipe; the local console path prints them in
//! full).
//!
//! - `wire` — pipe name, byte limits, UTF-16/UTF-8 line helpers
//! - `replies` — pending command ids the UI thread answers
//! - `security` — same-user ACL on the server, owner check on the client
//! - `server` — accept loop + `PipeCommand` dispatch
//! - `client` — forward a command to the resident, read its reply

mod client;
mod replies;
mod security;
mod server;
mod wire;

pub use client::{try_forward_patiently, try_forward_to_running, ForwardOutcome};
pub use replies::{set_reply, take_reply};
pub use server::{start_server, stop_server};
pub use wire::PIPE_NAME;

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

    #[test]
    fn replies_preserve_command_parser_failures_for_the_pipe_client() {
        assert_eq!(reply_for("list/actions"), "OK");
        assert!(reply_for("not-a-command").starts_with("ERROR: "));
    }
}
