//! Win32 constants kept as plain integers so hook math stays in
//! `u32`/`i32`/`usize` instead of the `windows` crate's newtype wrappers.
//! `WM_QUIT_LOOP` (== WM_QUIT) is the custom thread-quit message `stop`
//! posts to unwind the pump.

pub(super) const WH_KEYBOARD_LL: i32 = 13;
pub(super) const WH_MOUSE_LL: i32 = 14;
pub(super) const WM_KEYDOWN: u32 = 0x0100;
pub(super) const WM_KEYUP: u32 = 0x0101;
pub(super) const WM_SYSKEYDOWN: u32 = 0x0104;
pub(super) const WM_SYSKEYUP: u32 = 0x0105;
pub(super) const WM_MBUTTONDOWN: u32 = 0x0207;
pub(super) const WM_MBUTTONUP: u32 = 0x0208;
pub(super) const WM_LBUTTONDOWN: u32 = 0x0201;
pub(super) const WM_LBUTTONUP: u32 = 0x0202;
pub(super) const WM_QUIT_LOOP: u32 = 0x0012;
pub(super) const WM_TIMER: u32 = 0x0113;
pub(super) const ACTIVATION_TIMER_ID: usize = 1;
pub(super) const TIMEOUT_TIMER_ID: usize = 2;

pub(super) const VK_SHIFT: i32 = 0x10;
pub(super) const VK_CONTROL: i32 = 0x11;
pub(super) const VK_MENU: i32 = 0x12;
pub(super) const VK_LWIN: i32 = 0x5B;
pub(super) const VK_RWIN: i32 = 0x5C;
pub(super) const VK_ESCAPE: u32 = 0x1B;
pub(super) const VK_LSHIFT: i32 = 0xA0;
pub(super) const VK_RSHIFT: i32 = 0xA1;
pub(super) const VK_LCONTROL: i32 = 0xA2;
pub(super) const VK_RCONTROL: i32 = 0xA3;
pub(super) const VK_LMENU: i32 = 0xA4;
pub(super) const VK_RMENU: i32 = 0xA5;
