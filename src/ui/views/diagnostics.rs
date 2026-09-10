//! Diagnostics subsection for the Advanced section: recent failure log,
//! IPC/settings repro hints, and a Clear button.
//!
//! Reads the global [`crate::win::diagnostics`] ring buffer directly, so no
//! `State` plumbing is needed; the returned element owns all its text.

use iced::widget::{button, column, text};
use iced::Element;

use crate::ui::app::Message;
use crate::win::diagnostics::{self, VIEW_TAIL};

/// Newest-first tail of the diagnostics log with repro hints.
pub fn section() -> Element<'static, Message> {
    let entries = diagnostics::list();
    let mut log = column![text("Diagnostics").size(15)].spacing(6);
    log = log.push(
        text("Recent LoopW failures land here instead of failing silently (newest first).")
            .size(12),
    );
    if entries.is_empty() {
        log = log.push(text("No diagnostics recorded this session.").size(13));
    } else {
        for entry in entries.iter().rev().take(VIEW_TAIL) {
            let mut item = column![text(entry.headline()).size(12)].spacing(2);
            if !entry.detail.is_empty() {
                item = item.push(text(entry.detail.clone()).size(11));
            }
            log = log.push(item);
        }
        let older = entries.len().saturating_sub(VIEW_TAIL);
        if older > 0 {
            log = log.push(text(format!("…plus {older} older.")).size(11));
        }
        log = log.push(button("Clear diagnostics").on_press(Message::ClearDiagnostics));
    }
    log = log
        .push(
            text("Reproduce a pipe (IPC) issue: run LoopW.exe list/actions from a console while LoopW is running; failures appear here.")
                .size(12),
        )
        .push(
            text("Reproduce a settings issue: make %LOCALAPPDATA%\\LoopW\\settings.json read-only, change a setting, and watch for the save failure here.")
                .size(12),
        );
    log.into()
}
