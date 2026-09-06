//! CLI entry: `loopw [activate|list/...|direction/...|action/...]`.
//!
//! With no args the iced settings app starts. With one command arg the
//! command is forwarded to the running instance (or executed after startup).

use crate::core::commands::parse_command;
use crate::settings::persistence;

pub fn run() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return crate::ui::run_settings();
    }
    if args.len() == 1 {
        match parse_command(&args[0]) {
            Ok(cmd) => {
                // Single-instance forwarding lives in `crate::win::ipc`.
                // For now: try forwarding, otherwise handle locally.
                if crate::win::ipc::try_forward_to_running(&args[0]) {
                    return Ok(());
                }
                return handle_local(cmd);
            }
            Err(error) => {
                eprintln!("ERROR: {error}");
                std::process::exit(2);
            }
        }
    }
    eprintln!("ERROR: Expected one command, such as direction/right or list/actions.");
    std::process::exit(2);
}

fn handle_local(cmd: crate::core::commands::LoopCommand) -> iced::Result {
    use crate::core::commands::LoopCommand;
    match cmd {
        LoopCommand::ListActions => {
            println!("{}", crate::core::commands::format_actions());
            Ok(())
        }
        LoopCommand::ListKeybinds | LoopCommand::ListAll => {
            let settings = persistence::load();
            let binds: Vec<(u32, u32, crate::core::actions::WindowAction, bool, bool)> = settings
                .keybinds
                .iter()
                .map(|k| (k.modifiers, k.vk, k.action, k.cycle_enabled, k.bypass_trigger))
                .collect();
            if matches!(cmd, LoopCommand::ListKeybinds) {
                println!(
                    "{}",
                    crate::core::commands::format_keybinds(
                        &binds,
                        settings.trigger_modifiers,
                        settings.trigger_vk,
                        settings.trigger_modifier_side,
                    )
                );
            } else {
                println!(
                    "Actions:\n{}\n\nKeybinds:\n{}",
                    crate::core::commands::format_actions(),
                    crate::core::commands::format_keybinds(
                        &binds,
                        settings.trigger_modifiers,
                        settings.trigger_vk,
                        settings.trigger_modifier_side,
                    )
                );
            }
            Ok(())
        }
        // Window-applying commands need the resident runtime; boot it.
        LoopCommand::Activate | LoopCommand::Apply(_) => crate::ui::run_settings(),
    }
}
