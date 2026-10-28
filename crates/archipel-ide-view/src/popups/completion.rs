//! The rows of the completion popup (UI-POP-030..032, TUI-GUI-138, M17): the kind badge, the
//! label with the matched characters, the gray signature after it and the tail right aligned
//! (the return type of a member, the package of a class), and the footer notice.

use archipel_ide_core::CompletionSession;
use archipel_ide_lsp::completion::{self as ranking, kind_name};
use archipel_ide_lsp::lsp_types::CompletionItem;

/// The color family of a kind badge; the renderers map it to their palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeTone {
    Method,
    Field,
    Class,
    Interface,
    Variable,
    Neutral,
}

/// The badge of a kind: a letter in a round (GUI) or plain (terminal) badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindBadge {
    /// The letter.
    pub letter: char,
    /// The color family.
    pub tone: BadgeTone,
}

/// The badge of a kind name (`kind_name`), `None` for kinds without one (text).
pub fn kind_badge(kind: &str) -> Option<KindBadge> {
    let (letter, tone) = match kind {
        "method" | "function" | "constructor" => ('m', BadgeTone::Method),
        "field" => ('f', BadgeTone::Field),
        "property" => ('p', BadgeTone::Field),
        "variable" | "value" | "reference" => ('v', BadgeTone::Variable),
        "constant" | "enumMember" => ('c', BadgeTone::Field),
        "class" | "struct" => ('C', BadgeTone::Class),
        "interface" => ('I', BadgeTone::Interface),
        "enum" => ('E', BadgeTone::Class),
        "module" | "file" => ('M', BadgeTone::Neutral),
        "typeParameter" => ('T', BadgeTone::Interface),
        "snippet" => ('s', BadgeTone::Neutral),
        "keyword" => ('k', BadgeTone::Neutral),
        "event" | "operator" => ('o', BadgeTone::Neutral),
        _ => return None,
    };
    Some(KindBadge { letter, tone })
}

/// One row of the popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionRow {
    /// The kind name (`method`, `class`, ...).
    pub kind: &'static str,
    /// The label.
    pub label: String,
    /// Matched character indices of the label.
    pub matched: Vec<usize>,
    /// The signature after the label (`(String name)`).
    pub detail: Option<String>,
    /// The tail at the right edge: the type of a member, the package of a class.
    pub tail: Option<String>,
    /// Struck through.
    pub deprecated: bool,
}

impl CompletionRow {
    /// The badge of the row's kind.
    pub fn badge(&self) -> Option<KindBadge> {
        kind_badge(self.kind)
    }
}

/// The row of `item` with the matched characters of its label.
pub fn row_for(item: &CompletionItem, matched: &[usize]) -> CompletionRow {
    CompletionRow {
        kind: kind_name(item.kind),
        label: item.label.clone(),
        matched: matched.to_vec(),
        detail: ranking::label_detail(item).map(str::to_owned),
        tail: ranking::type_text(item)
            .filter(|_| item.label_details.is_some() || ranking::label_detail(item).is_none())
            .map(str::to_owned),
        deprecated: ranking::is_deprecated(item),
    }
}

/// The visible rows of `session`, best first.
pub fn rows(session: &CompletionSession) -> Vec<CompletionRow> {
    session
        .ranked
        .iter()
        .filter_map(|ranked| {
            let item = session.items.get(ranked.index)?;
            Some(row_for(item, &ranked.label_positions))
        })
        .collect()
}

/// "Indexing… results may be incomplete (jdtls: Building 0%)".
pub fn indexing_notice(status: &str) -> String {
    format!("Indexing… results may be incomplete ({status})")
}

/// The footer hint of the popup, without a notice; `mac` writes the keys as symbols.
pub fn footer_hint(mac: bool) -> &'static str {
    if mac {
        "⏎ inserts, ⇥ replaces the word at the caret, ⎋ closes"
    } else {
        "Enter inserts, Tab replaces the word at the caret, Esc closes"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(value: serde_json::Value) -> CompletionItem {
        serde_json::from_value(value).unwrap()
    }

    /// UI-POP-030: the signature follows the label, the type is right aligned.
    #[test]
    fn rows_use_label_details_for_signature_and_type() {
        let item = item(json!({
            "label": "greeting", "kind": 2, "detail": "App.greeting(String name) : String",
            "labelDetails": {"detail": "(String name)", "description": "String"}
        }));
        let row = row_for(&item, &[]);
        assert_eq!(row.detail.as_deref(), Some("(String name)"));
        assert_eq!(row.tail.as_deref(), Some("String"));
        assert_eq!(row.kind, "method");
        let plain = item_plain();
        assert_eq!(
            row_for(&plain, &[]).tail.as_deref(),
            Some("fn(&self) -> usize")
        );
    }

    fn item_plain() -> CompletionItem {
        item(json!({"label": "len", "detail": "fn(&self) -> usize"}))
    }

    /// A class from the native source carries its package as the description: the tail.
    #[test]
    fn a_class_row_has_its_package_as_the_tail() {
        let class = item(json!({
            "label": "ArrayList", "kind": 7,
            "labelDetails": {"description": "java.util"}
        }));
        let row = row_for(&class, &[0, 1]);
        assert_eq!(row.tail.as_deref(), Some("java.util"));
        assert_eq!(row.detail, None);
        assert_eq!(row.matched, vec![0, 1]);
    }

    #[test]
    fn badges_cover_the_kinds_with_a_letter() {
        assert_eq!(kind_badge("method").map(|b| b.letter), Some('m'));
        assert_eq!(kind_badge("class").map(|b| b.tone), Some(BadgeTone::Class));
        assert!(kind_badge("text").is_none());
    }

    #[test]
    fn indexing_notice_names_the_server_activity() {
        assert_eq!(
            indexing_notice("jdtls: Building 0%"),
            "Indexing… results may be incomplete (jdtls: Building 0%)"
        );
    }
}
