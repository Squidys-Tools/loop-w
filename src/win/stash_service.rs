//! Stash service: hide windows at screen edges, reveal on hover.
//!
//! Ports `WindowStashService` record management. Identity matching
//! (`crate::core::identity`) guarantees only unambiguous windows restore
//! after a restart; HWNDs alone are never trusted.

use crate::core::stash::StashRecord;

/// In-memory stash with persistence handled by settings save/load.
#[derive(Debug, Default)]
pub struct StashService {
    records: Vec<StashRecord>,
}

impl StashService {
    pub fn records(&self) -> &[StashRecord] {
        &self.records
    }

    pub fn remove_missing(&mut self, live_ids: &[String]) {
        self.records.retain(|r| live_ids.contains(&r.id));
    }
}
