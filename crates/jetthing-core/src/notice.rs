//! Completed-operation notices: what the UI shows as a balloon (a commit finished, a
//! toolchain installed). Commands push them; the UI drains them once per frame. The
//! text is written for the user and carries no command or component ids.

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Success,
    Info,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub body: String,
}

/// Notices waiting to be shown, oldest first.
#[derive(Debug, Default)]
pub struct Notices {
    queue: VecDeque<Notice>,
}

impl Notices {
    pub fn push(&mut self, kind: NoticeKind, title: impl Into<String>, body: impl Into<String>) {
        // The same notice twice in a row (a retried action) is shown once.
        let notice = Notice {
            kind,
            title: title.into(),
            body: body.into(),
        };
        if self.queue.back() != Some(&notice) {
            self.queue.push_back(notice);
        }
    }

    /// Takes every waiting notice.
    pub fn take(&mut self) -> Vec<Notice> {
        self.queue.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_are_taken_once_and_repeats_collapse() {
        let mut notices = Notices::default();
        notices.push(NoticeKind::Success, "Committed", "2 files");
        notices.push(NoticeKind::Success, "Committed", "2 files");
        notices.push(NoticeKind::Error, "Setup failed", "no network");
        assert_eq!(notices.take().len(), 2);
        assert!(notices.take().is_empty());
    }
}
