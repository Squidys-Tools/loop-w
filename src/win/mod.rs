//! Windows runtime: owner of all Win32 side effects.
//!
//! Each submodule is intentionally small. This crate targets Windows only;
//! the pure logic these services depend on lives in `crate::core` and is
//! fully unit tested.

pub mod actions_runtime;
pub mod display;
pub mod events;
pub mod hooks;
pub mod instance;
pub mod ipc;
pub mod monitor_service;
pub mod native;
pub mod overlay;
pub mod placement;
pub mod policy;
pub mod query;
pub mod shared;
pub mod snap_service;
pub mod startup;
pub mod stash_service;
pub mod target_frame;
pub mod tray;
