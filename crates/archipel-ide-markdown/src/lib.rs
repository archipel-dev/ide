//! UI-independent Markdown service: a CommonMark/GFM block model for the live preview,
//! selection formatting and insertion transforms, scroll mapping and HTML export.

pub mod format;
pub mod model;
pub mod scroll;

pub use model::{Align, Block, BlockKind, Document, Inline, ListItem, parse, to_html};

/// Whether `path` names a Markdown file.
pub fn is_markdown_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
}

/// Editor layout of a Markdown file (the top-right island).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    EditorOnly,
    #[default]
    Split,
    PreviewOnly,
}

impl Layout {
    pub fn id(self) -> &'static str {
        match self {
            Layout::EditorOnly => "editor",
            Layout::Split => "split",
            Layout::PreviewOnly => "preview",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "editor" => Some(Layout::EditorOnly),
            "split" => Some(Layout::Split),
            "preview" => Some(Layout::PreviewOnly),
            _ => None,
        }
    }

    pub fn shows_editor(self) -> bool {
        self != Layout::PreviewOnly
    }

    pub fn shows_preview(self) -> bool {
        self != Layout::EditorOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_ids_round_trip() {
        for layout in [Layout::EditorOnly, Layout::Split, Layout::PreviewOnly] {
            assert_eq!(Layout::from_id(layout.id()), Some(layout));
        }
        assert_eq!(Layout::from_id("bogus"), None);
        assert!(is_markdown_path(std::path::Path::new("A/README.MD")));
        assert!(!is_markdown_path(std::path::Path::new("a.rs")));
    }
}
