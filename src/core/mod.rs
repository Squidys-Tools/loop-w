//! Pure window-management logic ported from the C# implementation.
//!
//! Every module here is platform independent and fully unit tested.
//! Win32 interop lives in `crate::win`; iced rendering lives in `crate::ui`.

pub mod actions;
pub mod commands;
pub mod cycle;
pub mod drag_snap;
pub mod frame_math;
pub mod hotkey;
pub mod identity;
pub mod keybind_match;
pub mod monitor;
pub mod nav;
pub mod radial;
pub mod radial_targets;
pub mod rect;
pub mod stash;
pub mod stash_rebase;
