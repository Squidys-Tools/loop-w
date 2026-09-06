//! Windows runtime: owner of all Win32 side effects.
//!
//! Each submodule is intentionally small (<300 lines). Live Win32 calls are
//! `cfg(windows)`-gated so `cargo test` stays green on any host; pure logic
//! used by these services lives in `crate::core` and is tested there.

pub mod actions_runtime;
pub mod hooks;
pub mod ipc;
pub mod monitor_service;
pub mod policy;
pub mod query;
pub mod snap_service;
pub mod startup;
pub mod stash_service;
pub mod tray;
