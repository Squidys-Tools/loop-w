//! CLI entry + single-instance activation.
//!
//! Ports `App.OnStartup`: with args, forward to the resident instance and
//! print its reply; without a resident, become resident AND run the command
//! after startup. With no args, activate the resident or become resident
//! hidden (tray only).

use crate::win::native;

pub fn run() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        let command = args.join(" ");
        if let Some(reply) = crate::win::ipc::try_forward_to_running(&command) {
            native::print_cli_line(&reply);
            return Ok(());
        }
        match crate::win::instance::acquire() {
            Ok(None) => return Ok(()),
            Ok(Some(guard)) => {
                return crate::ui::run_daemon(Some(command), guard);
            }
            Err(_) => return Ok(()),
        }
    }
    match crate::win::instance::acquire() {
        Ok(None) => Ok(()),
        Ok(Some(guard)) => crate::ui::run_daemon(None, guard),
        Err(_) => Ok(()),
    }
}
