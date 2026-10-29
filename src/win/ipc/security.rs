//! Same-user pipe security: the SDDL descriptor the server pins on
//! `LoopW-Commands` and the owner check the client runs before trusting
//! a reply.

use windows::core::{BOOL, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo,
    SDDL_REVISION_1, SE_KERNEL_OBJECT,
};
use windows::Win32::Security::*;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use super::wire::wide_null;

/// SECURITY_ATTRIBUTES restricting the pipe to System + current user,
/// mirroring `PipeOptions.CurrentUserOnly`.
pub(super) struct PipeSecurity {
    pub(super) attributes: SECURITY_ATTRIBUTES,
    descriptor: PSECURITY_DESCRIPTOR,
    _sid_buffer: Vec<u8>,
}

impl Drop for PipeSecurity {
    fn drop(&mut self) {
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.descriptor.0)));
        }
    }
}

pub(super) fn current_user_sa() -> Option<PipeSecurity> {
    unsafe {
        // Current user SID -> string -> SDDL "O:<sid>D:P(A;;GA;;;SY)(A;;GA;;;<sid>)".
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return None;
        }
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        let mut buffer = vec![0u8; needed.max(64) as usize];
        if GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            needed,
            &mut needed,
        )
        .is_err()
        {
            let _ = CloseHandle(token);
            return None;
        }
        let _ = CloseHandle(token);
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let mut sid_string = windows::core::PWSTR::null();
        if ConvertSidToStringSidW(user.User.Sid, &mut sid_string).is_err() {
            return None;
        }
        let sid = sid_string.to_string().unwrap_or_default();
        let _ = LocalFree(Some(HLOCAL(sid_string.0 as *mut core::ffi::c_void)));
        if sid.is_empty() {
            return None;
        }
        // `O:` pins the owner to the user SID. Without it the pipe inherits the
        // creating token's default owner, which is BUILTIN\Administrators on an
        // elevated token, and the client's TokenUser comparison below would
        // reject our own pipe.
        let sddl = format!("O:{sid}D:P(A;;GA;;;SY)(A;;GA;;;{sid})");
        let sddl_wide = wide_null(&sddl);
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        let mut descriptor_size = 0u32;
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl_wide.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            Some(&mut descriptor_size),
        )
        .is_err()
        {
            return None;
        }
        let sa = SECURITY_ATTRIBUTES {
            nLength: core::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: BOOL(0),
        };
        Some(PipeSecurity {
            attributes: sa,
            descriptor,
            _sid_buffer: buffer,
        })
    }
}

/// True when `pipe` is owned by the current user.
/// Reads the pipe's owner SID and compares it with the process token user.
pub(super) fn pipe_owned_by_self(pipe: HANDLE) -> bool {
    use windows::Win32::Security::{EqualSid, OWNER_SECURITY_INFORMATION, PSID};
    unsafe {
        let mut owner: PSID = PSID::default();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        let status = GetSecurityInfo(
            pipe,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            None,
            None,
            Some(&mut descriptor),
        );
        if status != ERROR_SUCCESS {
            return false;
        }
        let owned = self_user_sid()
            .map(|mut sid| {
                EqualSid(owner, PSID(sid.as_mut_ptr() as *mut core::ffi::c_void)).is_ok()
            })
            .unwrap_or(false);
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        owned
    }
}

/// Current process user SID bytes (for owner comparison).
fn self_user_sid() -> Option<Vec<u8>> {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return None;
        }
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        let mut buffer = vec![0u8; needed.max(64) as usize];
        let capacity = buffer.len() as u32;
        let ok = GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            capacity,
            &mut needed,
        )
        .is_ok();
        let _ = CloseHandle(token);
        if !ok {
            return None;
        }
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let sid_ptr = user.User.Sid.0 as *const u8;
        if sid_ptr.is_null() {
            return None;
        }
        let length = GetLengthSid(user.User.Sid);
        if length == 0 || length > 256 {
            return None;
        }
        // Copy the SID bytes themselves (NOT the TOKEN_USER header: the
        // struct starts with the SID pointer + attributes).
        Some(core::slice::from_raw_parts(sid_ptr, length as usize).to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Storage::FileSystem::*;
    use windows::Win32::System::Pipes::*;

    /// Same-user pipe ownership roundtrip: a pipe carrying the production
    /// `current_user_sa` descriptor must verify as self-owned (guards the
    /// CurrentUserOnly client check).
    #[test]
    fn pipe_owner_verifies_self_owned_pipe() {
        let name = format!(
            r"\\.\pipe\LoopW-Debug-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let wide = wide_null(&name);
        let security = current_user_sa().expect("current-user security descriptor");
        let owned = unsafe {
            let server = CreateNamedPipeW(
                PCWSTR(wide.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                4096,
                4096,
                0,
                Some(&security.attributes as *const SECURITY_ATTRIBUTES),
            );
            assert!(!server.is_invalid(), "create pipe");
            let client = CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
            .expect("open pipe");
            let owned = pipe_owned_by_self(client);
            let _ = CloseHandle(client);
            let _ = CloseHandle(server);
            owned
        };
        // The descriptor pins the owner, so this holds on an elevated token
        // where the default owner would be BUILTIN\Administrators.
        assert!(owned, "owner check fails on our own production pipe");
    }
}
