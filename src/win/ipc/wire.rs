//! Wire protocol shared by the pipe server and client: pipe name,
//! 256-char / 2 s limits, UTF-16 name conversion, lossy line decode.

pub const PIPE_NAME: &str = r"\\.\pipe\LoopW-Commands";
pub(super) const MAX_COMMAND_CHARS: usize = 256;
pub(super) const READ_TIMEOUT_MS: u64 = 2000;

pub(super) fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(core::iter::once(0)).collect()
}

pub(super) fn pipe_name_wide() -> Vec<u16> {
    wide_null(PIPE_NAME)
}

pub(super) fn decode(bytes: &[u8]) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    while text.ends_with('\r') {
        text.pop();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_decode_trims_cr_and_replaces_invalid_utf8() {
        assert_eq!(decode(b"activate\r"), "activate");
        assert_eq!(decode(b"bad\xFF"), "bad\u{FFFD}");
    }
}
