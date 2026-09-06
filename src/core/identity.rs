//! Window identity matching for safe stash restore.
//!
//! The executable path anchors a match; two additional hints (process id,
//! window class, title) are required so a reused HWND or generic title can
//! never restore an unrelated window on its own.

use super::stash::StashRecord;

/// Runtime identity of a candidate window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowIdentity {
    pub executable_path: String,
    pub process_id: u32,
    pub window_class: String,
    pub title: String,
}

pub fn is_match(record: &StashRecord, candidate: &WindowIdentity) -> bool {
    if record.executable_path.trim().is_empty()
        || !record
            .executable_path
            .eq_ignore_ascii_case(&candidate.executable_path)
    {
        return false;
    }
    let mut hints = 0;
    if record.process_id != 0 && record.process_id == candidate.process_id {
        hints += 1;
    }
    if !record.window_class.trim().is_empty()
        && record
            .window_class
            .eq_ignore_ascii_case(&candidate.window_class)
    {
        hints += 1;
    }
    if !record.title.is_empty() && record.title == candidate.title {
        hints += 1;
    }
    hints >= 2
}

pub fn is_same_runtime_window(record: &StashRecord, candidate: &WindowIdentity) -> bool {
    if record.process_id == 0
        || record.process_id != candidate.process_id
        || !record
            .executable_path
            .eq_ignore_ascii_case(&candidate.executable_path)
    {
        return false;
    }
    !record.window_class.trim().is_empty()
        && record
            .window_class
            .eq_ignore_ascii_case(&candidate.window_class)
}

/// Find the single unambiguous match, if exactly one candidate matches.
pub fn find_unambiguous_match<T: Clone>(
    record: &StashRecord,
    candidates: &[(T, WindowIdentity)],
) -> Option<T> {
    let mut found: Option<T> = None;
    for (value, identity) in candidates {
        if !is_match(record, identity) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(value.clone());
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::stash::{StashEdge, StashMonitor, StashPlacement, StashRect};

    fn record() -> StashRecord {
        StashRecord {
            id: "r1".to_string(),
            executable_path: "C:\\Apps\\Note.exe".to_string(),
            process_id: 1234,
            window_class: "Notepad".to_string(),
            title: "Notes".to_string(),
            edge: StashEdge::Left,
            original_placement: StashPlacement::default(),
            original_monitor: StashMonitor::default(),
            stashed_frame: StashRect { left: 0, top: 0, right: 8, bottom: 100 },
        }
    }

    fn identity() -> WindowIdentity {
        WindowIdentity {
            executable_path: "c:\\apps\\note.exe".to_string(),
            process_id: 1234,
            window_class: "notepad".to_string(),
            title: "Notes".to_string(),
        }
    }

    #[test]
    fn rejects_ambiguity() {
        let rec = record();
        assert!(is_match(&rec, &identity()));
        // Only one hint (class) — title differs and pid differs.
        let weak = WindowIdentity {
            process_id: 9999,
            title: "Other".to_string(),
            ..identity()
        };
        assert!(!is_match(&rec, &weak));
        // Two identical candidates are ambiguous.
        let candidates = vec![(1u32, identity()), (2u32, identity())];
        assert_eq!(find_unambiguous_match(&rec, &candidates), None);
        let single = vec![(1u32, identity())];
        assert_eq!(find_unambiguous_match(&rec, &single), Some(1));
    }
}
