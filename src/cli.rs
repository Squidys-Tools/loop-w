//! CLI entry + single-instance activation.
//!
//! Ports `App.OnStartup`: with args, forward to the resident instance and
//! print its reply; without a resident, become resident AND run the command
//! after startup. With no args, activate the resident or become resident
//! hidden (tray only).

use crate::win::ipc::ForwardOutcome;
use crate::win::native;

pub fn run() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        let command = args.join(" ");
        match crate::win::ipc::try_forward_to_running(&command) {
            ForwardOutcome::Replied(reply) => {
                native::print_cli_line(&reply);
                return Ok(());
            }
            ForwardOutcome::DeliveredNoReply => {
                // Delivered but the reply was lost — a second forward would
                // run a non-idempotent command twice.
                native::print_cli_line(
                    "LoopW received the command but did not reply in time. It was not sent again.",
                );
                return Ok(());
            }
            ForwardOutcome::NotDelivered => {}
        }
        // Mutex may already be owned while the pipe is still binding — the
        // resident won the race moments ago. `acquire(Some)` deliberately does
        // not substitute `activate` for our command; we patient-forward here
        // so the original command (and its reply) is not dropped.
        match crate::win::instance::acquire(Some(&command)) {
            Ok(None) => {
                match crate::win::ipc::try_forward_patiently(&command) {
                    ForwardOutcome::Replied(reply) => native::print_cli_line(&reply),
                    ForwardOutcome::DeliveredNoReply => native::print_cli_line(
                        "LoopW received the command but did not reply in time. It was not sent again.",
                    ),
                    ForwardOutcome::NotDelivered => native::print_cli_line(
                        "LoopW is still starting — the command could not be delivered. Retry in a moment.",
                    ),
                }
                return Ok(());
            }
            Ok(Some(guard)) => {
                return crate::ui::run_daemon(Some(command), guard);
            }
            Err(_) => return Ok(()),
        }
    }
    match crate::win::instance::acquire(None) {
        Ok(None) => Ok(()),
        Ok(Some(guard)) => crate::ui::run_daemon(None, guard),
        Err(_) => Ok(()),
    }
}
