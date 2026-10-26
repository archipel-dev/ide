//! Converts the documentation shapes LSP allows into one Markdown string.

use lsp_types::{
    CompletionItem, Documentation, Hover, HoverContents, MarkedString, MarkupContent, MarkupKind,
};

fn marked_string(value: &MarkedString) -> String {
    match value {
        MarkedString::String(text) => text.clone(),
        MarkedString::LanguageString(code) => {
            format!("```{}\n{}\n```", code.language, code.value)
        }
    }
}

fn markup(content: &MarkupContent) -> String {
    match content.kind {
        MarkupKind::Markdown => content.value.clone(),
        // Plain text must not be interpreted as Markdown.
        MarkupKind::PlainText => escape_markdown(&content.value),
    }
}

/// Escapes Markdown control characters so plain text renders verbatim.
pub fn escape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(
            c,
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '<' | '>' | '#' | '|' | '~'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Markdown for a hover response; empty when the server sent nothing useful.
pub fn hover_markdown(hover: &Hover) -> String {
    let text = match &hover.contents {
        HoverContents::Scalar(value) => marked_string(value),
        HoverContents::Array(values) => values
            .iter()
            .map(marked_string)
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n"),
        HoverContents::Markup(content) => markup(content),
    };
    text.trim().to_owned()
}

/// Markdown for a completion item's documentation panel: the detail as a
/// code header, then the documentation.
pub fn completion_markdown(item: &CompletionItem, language: &str) -> Option<String> {
    let documentation = item.documentation.as_ref().map(|doc| match doc {
        Documentation::String(text) => escape_markdown(text),
        Documentation::MarkupContent(content) => markup(content),
    });
    let detail = item
        .detail
        .as_deref()
        .filter(|detail| !detail.trim().is_empty())
        .map(|detail| format!("```{language}\n{detail}\n```"));
    match (detail, documentation.filter(|doc| !doc.trim().is_empty())) {
        (None, None) => None,
        (Some(detail), None) => Some(detail),
        (None, Some(doc)) => Some(doc),
        (Some(detail), Some(doc)) => Some(format!("{detail}\n\n{doc}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::LanguageString;

    #[test]
    fn hover_shapes_become_markdown() {
        let markup = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: " **bold** ".into(),
            }),
            range: None,
        };
        assert_eq!(hover_markdown(&markup), "**bold**");
        let array = Hover {
            contents: HoverContents::Array(vec![
                MarkedString::LanguageString(LanguageString {
                    language: "java".into(),
                    value: "int x".into(),
                }),
                MarkedString::String("".into()),
                MarkedString::String("doc".into()),
            ]),
            range: None,
        };
        assert_eq!(hover_markdown(&array), "```java\nint x\n```\n\n---\n\ndoc");
        let plain = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::PlainText,
                value: "a*b".into(),
            }),
            range: None,
        };
        assert_eq!(hover_markdown(&plain), "a\\*b");
    }

    #[test]
    fn completion_docs_combine_detail_and_documentation() {
        let mut item = CompletionItem {
            label: "greeting".into(),
            detail: Some("String greeting(String name)".into()),
            ..Default::default()
        };
        assert_eq!(
            completion_markdown(&item, "java").unwrap(),
            "```java\nString greeting(String name)\n```"
        );
        item.documentation = Some(Documentation::String("Says hi".into()));
        assert!(
            completion_markdown(&item, "java")
                .unwrap()
                .ends_with("\n\nSays hi")
        );
        assert!(completion_markdown(&CompletionItem::default(), "java").is_none());
    }
}
