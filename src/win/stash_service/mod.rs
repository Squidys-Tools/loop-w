//! Stash service: hide windows at screen edges, reveal on hover.
//!
//! Ports `WindowStashService`: edge choice by nearest work-area edge,
//! peek-strip frames, hover hit-zones with reveal-delay arming, FIFO
//! reveal-next, DPI/work-area rebasing, unambiguous startup restore, and
//! persistence of only fully-identified records. There are no strip
//! windows — hover is purely geometric.

mod convert;
mod hover;
mod persist;
mod records;
mod reveal;
mod stash;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use windows::Win32::UI::WindowsAndMessaging::WINDOWPLACEMENT;

use crate::core::identity::WindowIdentity;
use crate::core::rect::Rect;
use crate::core::stash::{StashEdge, StashMonitor};

pub use hover::{is_active, poll, reveal_at_cursor};
pub use persist::{handle_settings_changed, restore_persisted};
pub use records::prune_stale;
pub use reveal::{restore_all, reveal_next};
pub use stash::stash;

struct LiveRecord {
    edge: StashEdge,
    placement: WINDOWPLACEMENT,
    monitor: StashMonitor,
    frame: Rect,
    identity: WindowIdentity,
    persisted_id: String,
}

struct StashLive {
    by_hwnd: HashMap<u64, LiveRecord>,
    order: Vec<u64>,
    pending: Option<(u64, u64)>,
    last_poll: u64,
}

static LIVE: OnceLock<Mutex<StashLive>> = OnceLock::new();

fn live() -> &'static Mutex<StashLive> {
    LIVE.get_or_init(|| {
        Mutex::new(StashLive {
            by_hwnd: HashMap::new(),
            order: Vec::new(),
            pending: None,
            last_poll: 0,
        })
    })
}
