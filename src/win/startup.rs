//! Launch-at-login via HKCU\...\Run (value `LoopW`).
//!
//! Ports `StartupManager`: quoted exe path, missing-value delete is fine,
//! open failure is false, all errors collapse to false.

use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::System::Registry::*;

/// Registry value name shared with the C# `StartupManager`.
pub const RUN_VALUE_NAME: &str = "LoopW";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(core::iter::once(0)).collect()
}

/// Enable or disable launch at login.
pub fn set_launch_at_login(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|error| format!("Could not locate the executable: {error}"))?;
    let exe = exe.to_string_lossy();
    if exe.trim().is_empty() {
        return Err("Could not locate the executable.".to_string());
    }
    unsafe {
        let key_path = wide_null(RUN_KEY);
        let mut key = HKEY::default();
        let status = RegCreateKeyW(HKEY_CURRENT_USER, PCWSTR(key_path.as_ptr()), &mut key);
        if status != ERROR_SUCCESS {
            return Err("Could not open the startup registry key.".to_string());
        }
        let result = if enabled {
            let value_name = wide_null(RUN_VALUE_NAME);
            let quoted = format!("\"{exe}\"");
            let data: Vec<u16> = quoted.encode_utf16().chain(core::iter::once(0)).collect();
            let bytes = core::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * core::mem::size_of::<u16>(),
            );
            RegSetValueExW(
                key,
                PCWSTR(value_name.as_ptr()),
                Some(0),
                REG_SZ,
                Some(bytes),
            )
        } else {
            let value_name = wide_null(RUN_VALUE_NAME);
            let status = RegDeleteValueW(key, PCWSTR(value_name.as_ptr()));
            if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
                ERROR_SUCCESS
            } else {
                status
            }
        };
        let _ = RegCloseKey(key);
        if result == ERROR_SUCCESS {
            Ok(())
        } else {
            Err("Could not update the startup registry value.".to_string())
        }
    }
}
