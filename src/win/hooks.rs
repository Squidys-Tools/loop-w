//! Global trigger + keybind hooks (low-level keyboard/mouse).
//!
//! Ports `GlobalHotkey` staging: capture mode with Esc cancel and reserved-key
//! rejection lives here; the iced settings UI drives it via messages.

/// Reserved keys that can never become the trigger.
pub fn is_reserved_vk(vk: u32) -> bool {
    matches!(vk, 0x1B) // Esc cancels capture; never a trigger.
}

/// Validate a capture result before committing it to settings.
pub fn validate_trigger(vk: u32) -> Result<u32, &'static str> {
    if vk == 0 {
        return Err("Press a key or key combination. Esc cancels.");
    }
    if is_reserved_vk(vk) {
        return Err("That key is reserved by Windows. Choose another trigger.");
    }
    Ok(vk)
}
