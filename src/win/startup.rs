//! Launch-at-login via the Windows Run registry key.

/// Registry value name shared with the C# `StartupManager`.
pub const RUN_VALUE_NAME: &str = "LoopW";

/// Enable or disable launch at login. Live registry writes on Windows.
pub fn set_launch_at_login(_enabled: bool) -> Result<(), String> {
    // Live HKCU\...\Run wiring lands with the Win32 backend.
    Ok(())
}
