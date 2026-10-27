//! Interactive snippet ("live template") session after a completion inserts
//! a snippet with tab stops: Tab/Shift+Tab move between stops, every stop's
//! text is selected so typing replaces it, and mirrored stops are edited
//! together through multi-cursor selections.

use crate::IdeState;
use archipel_ide_lsp::snippet::Snippet;
use archipel_ide_text::Selection;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetSession {
    /// Project-relative document path.
    pub path: PathBuf,
    /// Absolute char ranges of each stop, in visiting order.
    pub stops: Vec<Vec<(usize, usize)>>,
    /// Index into `stops`.
    pub current: usize,
    /// Whether the last stop is the snippet's `$0`.
    final_stop: bool,
    /// Document length (chars) the ranges refer to.
    len: usize,
}

impl SnippetSession {
    /// Moves every stop by `delta` chars (text was inserted before the snippet).
    pub(crate) fn shift(&mut self, delta: isize) {
        for range in self.stops.iter_mut().flatten() {
            range.0 = range.0.saturating_add_signed(delta);
            range.1 = range.1.saturating_add_signed(delta);
        }
        self.len = self.len.saturating_add_signed(delta);
    }

    /// Ranges of every stop, for highlighting.
    pub fn ranges(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.stops.iter().flatten().copied()
    }
}

impl IdeState {
    /// Starts a session for `snippet` inserted at `offset` in the active document.
    pub(crate) fn start_snippet(&mut self, snippet: &Snippet, offset: usize) {
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
        else {
            return;
        };
        let stops = snippet
            .stops
            .iter()
            .map(|ranges| {
                ranges
                    .iter()
                    .map(|&(start, end)| (start + offset, end + offset))
                    .collect()
            })
            .collect();
        self.snippet = Some(SnippetSession {
            path: document.path.clone(),
            stops,
            current: 0,
            final_stop: snippet.has_final_stop,
            len: document.text.len_chars(),
        });
        self.select_snippet_stop();
    }

    /// Selects every range of the current stop.
    fn select_snippet_stop(&mut self) {
        let Some(session) = &self.snippet else {
            return;
        };
        let Some(ranges) = session.stops.get(session.current).cloned() else {
            return;
        };
        let last = session.current + 1 == session.stops.len();
        let finishing = last && session.final_stop;
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get_mut(index))
        else {
            return;
        };
        let len = document.text.len_chars();
        // The caret sits in the first occurrence of the field; the mirrors follow it
        // (ED-TMPL-002).
        let first = ranges.iter().copied().min().unwrap_or((0, 0));
        document.selections.set_single(Selection {
            anchor: first.0.min(len),
            head: first.1.min(len),
        });
        if finishing {
            // `$0` is where the caret ends; the session is over.
            let caret = document.selections.primary().head;
            document.selections.set_single(Selection::caret(caret));
            self.snippet = None;
        }
    }

    /// Moves to the next stop (Tab); leaving the last stop ends the session.
    pub fn snippet_next(&mut self) -> bool {
        let Some(session) = self.snippet.as_mut() else {
            return false;
        };
        if session.current + 1 >= session.stops.len() {
            self.exit_snippet();
            return true;
        }
        session.current += 1;
        self.select_snippet_stop();
        true
    }

    /// Moves to the previous stop (Shift+Tab).
    pub fn snippet_previous(&mut self) -> bool {
        let Some(session) = self.snippet.as_mut() else {
            return false;
        };
        if session.current > 0 {
            session.current -= 1;
            self.select_snippet_stop();
        }
        true
    }

    /// Ends the session; the caret stays where it is.
    pub fn exit_snippet(&mut self) {
        if self.snippet.take().is_some()
            && let Some(document) = self
                .active_document
                .and_then(|index| self.documents.get_mut(index))
        {
            let head = document.selections.primary().head;
            document.selections.set_single(Selection::caret(head));
        }
    }

    /// Re-maps stop ranges after the user edited the active document and copies the edited field
    /// into its mirrors. Edits are expected inside the first occurrence of the current stop; any
    /// other change, or a caret outside it, ends the session (ED-TMPL-033).
    pub fn snippet_after_change(&mut self) {
        let Some(session) = self.snippet.as_ref() else {
            return;
        };
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get_mut(index))
            .filter(|document| document.path == session.path)
        else {
            self.snippet = None;
            return;
        };
        let len = document.text.len_chars();
        let delta = len as isize - session.len as isize;
        let mut current: Vec<(usize, usize)> = session.stops[session.current].clone();
        current.sort_unstable();
        let primary = current[0];
        let primary_end = primary.1.saturating_add_signed(delta);
        let selections = document.selections.selections().to_vec();
        let inside = selections.len() == 1
            && primary_end >= primary.0
            && (primary.0..=primary_end).contains(&selections[0].start())
            && (primary.0..=primary_end).contains(&selections[0].end());
        if !inside {
            self.snippet = None;
            return;
        }
        // Ranges after the edited field move with it.
        let shift = |position: usize| {
            if position >= primary.1 && delta != 0 {
                position.saturating_add_signed(delta)
            } else {
                position
            }
        };
        let mut stops: Vec<Vec<(usize, usize)>> = session
            .stops
            .iter()
            .enumerate()
            .map(|(index, ranges)| {
                ranges
                    .iter()
                    .map(|&range| {
                        if index == session.current && range == primary {
                            (primary.0, primary_end)
                        } else {
                            (shift(range.0), shift(range.1))
                        }
                    })
                    .collect()
            })
            .collect();
        // The mirrors of the current field show the same text.
        let text = document
            .text
            .slice(primary.0, primary_end)
            .unwrap_or_default();
        let mut changes = Vec::new();
        for &(start, end) in stops[session.current]
            .iter()
            .filter(|r| **r != (primary.0, primary_end))
        {
            if document.text.slice(start, end).unwrap_or_default() != text {
                changes.push(archipel_ide_text::TextChange {
                    start_char: start,
                    end_char: end,
                    inserted: text.clone(),
                });
            }
        }
        if !changes.is_empty() {
            changes.sort_by_key(|change| change.start_char);
            let before = document.selections.selections().to_vec();
            let result = crate::editor_input::apply_changes(
                document,
                archipel_ide_text::UndoKind::Typing,
                changes.clone(),
                |changes, _| {
                    before
                        .iter()
                        .map(|s| Selection {
                            anchor: crate::editor_input::map_through(
                                changes,
                                s.anchor,
                                crate::editor_input::Bias::Right,
                            ),
                            head: crate::editor_input::map_through(
                                changes,
                                s.head,
                                crate::editor_input::Bias::Right,
                            ),
                        })
                        .collect()
                },
            );
            if result.is_err() {
                self.snippet = None;
                return;
            }
            let mirrors: std::collections::HashSet<(usize, usize)> =
                changes.iter().map(|c| (c.start_char, c.end_char)).collect();
            for ranges in stops.iter_mut() {
                for range in ranges.iter_mut() {
                    if let Some(change) = changes
                        .iter()
                        .find(|c| mirrors.contains(range) && (c.start_char, c.end_char) == *range)
                    {
                        let start = crate::editor_input::map_through(
                            &changes,
                            range.0,
                            crate::editor_input::Bias::Left,
                        );
                        *range = (start, start + change.inserted.chars().count());
                    } else {
                        *range = (
                            crate::editor_input::map_through(
                                &changes,
                                range.0,
                                crate::editor_input::Bias::Right,
                            ),
                            crate::editor_input::map_through(
                                &changes,
                                range.1,
                                crate::editor_input::Bias::Right,
                            ),
                        );
                    }
                }
            }
        }
        let len = document.text.len_chars();
        if let Some(session) = self.snippet.as_mut() {
            session.stops = stops;
            session.len = len;
        }
    }

    /// Ends the session if the caret left the current stop (click, arrows).
    pub fn snippet_after_caret_move(&mut self) {
        let Some(session) = self.snippet.as_ref() else {
            return;
        };
        let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get(index))
            .filter(|document| document.path == session.path)
        else {
            self.snippet = None;
            return;
        };
        let head = document.selections.primary().head;
        let inside = session.stops[session.current]
            .iter()
            .any(|&(start, end)| (start..=end).contains(&head));
        if !inside {
            self.snippet = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{IdeState, built_in_commands};
    use archipel_ide_lsp::snippet;
    use archipel_ide_text::Selection;
    use serde_json::json;

    fn state_with(text: &str, name: &str) -> (IdeState, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-snippet-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), text).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "a.txt"}))
            .unwrap();
        (state, root)
    }

    fn insert_snippet(state: &mut IdeState, at: usize, body: &str) {
        let parsed = snippet::parse(body, &());
        state.documents[0].text.edit(at, at, &parsed.text).unwrap();
        state.start_snippet(&parsed, at);
    }

    fn type_text(state: &mut IdeState, text: &str) {
        let document = &mut state.documents[0];
        document
            .selections
            .insert(&mut document.text, text)
            .unwrap();
        state.snippet_after_change();
    }

    #[test]
    fn tab_walks_stops_and_typing_replaces_placeholders() {
        let (mut state, root) = state_with("x = ;", "walk");
        insert_snippet(&mut state, 4, "greeting(${1:name}, ${2:count})$0");
        assert_eq!(state.documents[0].text.text(), "x = greeting(name, count);");
        let primary = state.documents[0].selections.primary();
        assert_eq!((primary.start(), primary.end()), (13, 17));
        type_text(&mut state, "who");
        assert_eq!(state.documents[0].text.text(), "x = greeting(who, count);");
        assert!(state.snippet_next());
        let primary = state.documents[0].selections.primary();
        assert_eq!(
            state.documents[0]
                .text
                .slice(primary.start(), primary.end())
                .unwrap(),
            "count"
        );
        assert!(state.snippet_previous());
        let primary = state.documents[0].selections.primary();
        assert_eq!((primary.start(), primary.end()), (13, 16));
        state.snippet_next();
        state.snippet_next();
        // `$0` after the closing parenthesis ends the session there.
        assert!(state.snippet.is_none());
        assert_eq!(state.documents[0].selections.primary().head, 24);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mirrored_stops_are_edited_together() {
        let (mut state, root) = state_with("", "mirror");
        insert_snippet(&mut state, 0, "let ${1:x} = 1; use($1);");
        assert_eq!(state.documents[0].text.text(), "let x = 1; use(x);");
        // One caret in the first occurrence; the mirror follows it (ED-TMPL-002).
        assert_eq!(state.documents[0].selections.selections().len(), 1);
        type_text(&mut state, "total");
        assert_eq!(state.documents[0].text.text(), "let total = 1; use(total);");
        assert!(state.snippet.is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn moving_away_or_escape_ends_the_session() {
        let (mut state, root) = state_with("", "escape");
        insert_snippet(&mut state, 0, "f(${1:a}, ${2:b})");
        state.documents[0]
            .selections
            .set_single(Selection::caret(0));
        state.snippet_after_caret_move();
        assert!(state.snippet.is_none());
        insert_snippet(&mut state, 0, "g(${1:a})");
        let commands = built_in_commands();
        commands
            .execute("snippet.exit", &mut state, json!({}))
            .unwrap();
        assert!(state.snippet.is_none());
        assert!(state.documents[0].selections.primary().is_caret());
        std::fs::remove_dir_all(root).unwrap();
    }
}
