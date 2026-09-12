//! `LoopW.exe <command>` parsing and list formatting.
//!
//! Ports `LoopCommandParser` / `LoopCommandFormatter`, including direction
//! aliases and the single-token command contract.

use super::actions::WindowAction;
use super::hotkey::{hotkey_name, TriggerModifierSide};

/// A parsed CLI / named-pipe command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopCommand {
    Activate,
    Apply(WindowAction),
    ListActions,
    ListKeybinds,
    ListAll,
}

/// Parse one raw command token.
pub fn parse_command(raw: &str) -> Result<LoopCommand, String> {
    let mut parts = raw.split_whitespace();
    let Some(token) = parts.next() else {
        return Err("Expected one command, such as direction/right or list/actions.".to_string());
    };
    if parts.next().is_some() {
        return Err("Expected one command, such as direction/right or list/actions.".to_string());
    }
    if token.eq_ignore_ascii_case("activate") {
        return Ok(LoopCommand::Activate);
    }
    if token.eq_ignore_ascii_case("list/actions") {
        return Ok(LoopCommand::ListActions);
    }
    if token.eq_ignore_ascii_case("list/keybinds") {
        return Ok(LoopCommand::ListKeybinds);
    }
    if token.eq_ignore_ascii_case("list/all") {
        return Ok(LoopCommand::ListAll);
    }
    // Case-insensitive direction/action prefix check without allocating a
    // lowercased copy of the command token.
    if let Some(body) = strip_ascii_prefix(token, "direction/") {
        if let Some(action) = parse_direction(body) {
            return Ok(LoopCommand::Apply(action));
        }
        return Err(format!(
            "Unknown direction '{}'. Use left, right, top, bottom, next, previous, or a directional screen command.",
            &token["direction/".len()..]
        ));
    }
    if let Some(name) = strip_ascii_prefix(token, "action/") {
        if let Some(action) = WindowAction::parse_token(name) {
            return Ok(LoopCommand::Apply(action));
        }
        return Err(format!(
            "Unknown action '{name}'. Use list/actions to see valid actions."
        ));
    }
    Err(format!(
        "Unknown command '{token}'. Use direction/<name>, action/<name>, or list/<scope>."
    ))
}

fn parse_direction(name: &str) -> Option<WindowAction> {
    if name.eq_ignore_ascii_case("left") {
        Some(WindowAction::LeftHalf)
    } else if name.eq_ignore_ascii_case("right") {
        Some(WindowAction::RightHalf)
    } else if name.eq_ignore_ascii_case("top") {
        Some(WindowAction::TopHalf)
    } else if name.eq_ignore_ascii_case("bottom") {
        Some(WindowAction::BottomHalf)
    } else if name.eq_ignore_ascii_case("next") {
        Some(WindowAction::NextScreen)
    } else if name.eq_ignore_ascii_case("previous") || name.eq_ignore_ascii_case("prev") {
        Some(WindowAction::PreviousScreen)
    } else if name.eq_ignore_ascii_case("left-screen") {
        Some(WindowAction::LeftScreen)
    } else if name.eq_ignore_ascii_case("right-screen") {
        Some(WindowAction::RightScreen)
    } else if name.eq_ignore_ascii_case("top-screen") {
        Some(WindowAction::TopScreen)
    } else if name.eq_ignore_ascii_case("bottom-screen") {
        Some(WindowAction::BottomScreen)
    } else {
        None
    }
}

fn strip_ascii_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let head = value.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        value.get(prefix.len()..)
    } else {
        None
    }
}

/// One line per action: `action/<token> - <name>`.
/// Lines join with CRLF, matching Environment.NewLine on Windows.
pub fn format_actions() -> String {
    WindowAction::ALL
        .iter()
        .map(|a| format!("action/{} - {}", a.token(), a.display_name()))
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// Keybind listing used by `list/keybinds`.
pub fn format_keybinds(
    keybinds: &[(u32, u32, WindowAction, bool, bool)],
    trigger_modifiers: u32,
    trigger_vk: u32,
    side: TriggerModifierSide,
) -> String {
    let mut lines = vec![format!(
        "trigger: {}",
        hotkey_name(trigger_modifiers, trigger_vk, side)
    )];
    if keybinds.is_empty() {
        lines.push("keybinds: none".to_string());
        return lines.join("\r\n");
    }
    for (modifiers, vk, action, cycle, bypass) in keybinds {
        let mut line = format!(
            "{} -> action/{}",
            hotkey_name(*modifiers, *vk, TriggerModifierSide::Any),
            action.token()
        );
        if *cycle {
            line.push_str(" (cycle)");
        }
        if *bypass {
            line.push_str(" (bypass trigger)");
        }
        lines.push(line);
    }
    lines.join("\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_direction_aliases() {
        assert_eq!(
            parse_command("direction/right"),
            Ok(LoopCommand::Apply(WindowAction::RightHalf))
        );
        assert_eq!(
            parse_command("direction/next"),
            Ok(LoopCommand::Apply(WindowAction::NextScreen))
        );
        assert_eq!(
            parse_command("direction/prev"),
            Ok(LoopCommand::Apply(WindowAction::PreviousScreen))
        );
    }

    #[test]
    fn maps_action_names() {
        assert_eq!(
            parse_command("action/maximize"),
            Ok(LoopCommand::Apply(WindowAction::Maximize))
        );
        assert!(parse_command("action/nope").is_err());
    }

    #[test]
    fn maps_activation_and_lists() {
        assert_eq!(parse_command("activate"), Ok(LoopCommand::Activate));
        assert_eq!(parse_command("list/actions"), Ok(LoopCommand::ListActions));
        assert_eq!(
            parse_command("list/keybinds"),
            Ok(LoopCommand::ListKeybinds)
        );
        assert_eq!(parse_command("list/all"), Ok(LoopCommand::ListAll));
    }

    #[test]
    fn rejects_malformed_commands() {
        assert!(parse_command("").is_err());
        assert!(parse_command("a b").is_err());
        assert!(parse_command("direction/upside-down").is_err());
        assert!(parse_command("bogus").is_err());
    }

    #[test]
    fn accepts_mixed_case_without_lowercasing_the_command() {
        assert_eq!(
            parse_command("DiReCtIoN/RiGhT"),
            Ok(LoopCommand::Apply(WindowAction::RightHalf))
        );
        assert_eq!(
            parse_command("AcTiOn/LeFt-HaLf"),
            Ok(LoopCommand::Apply(WindowAction::LeftHalf))
        );
    }

    #[test]
    fn actions_cover_every_action() {
        let text = format_actions();
        for action in WindowAction::ALL {
            assert!(
                text.contains(&format!("action/{}", action.token())),
                "missing {action:?}"
            );
        }
    }
}
