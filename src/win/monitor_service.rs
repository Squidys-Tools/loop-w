//! Cached monitor enumeration + DPI-aware work areas.
//!
//! Ports `MonitorService` caching (generation counter + snapshot cache). Live
//! `EnumDisplayMonitors` calls are Windows-only; padding/translation math is
//! tested in `crate::core::monitor`.

use crate::core::monitor::MonitorSnapshot;

/// Cache generation; bumped whenever displays, DPI, or padding change.
#[derive(Debug, Default)]
pub struct MonitorCache {
    generation: u64,
    monitors: Vec<MonitorSnapshot>,
}

impl MonitorCache {
    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.monitors.clear();
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}
