//! The Markdown pack: language, file type and parser. `markdown_inline` is the dependent
//! language of the inline grammar (PLAT-LANG-004): only reached through injection, so it has
//! no file type. The editor, split preview and renderer live in `archipel-ide-markdown`.

use crate::{LanguagePack, PackContext};

pub struct MarkdownPack;

impl LanguagePack for MarkdownPack {
    fn id(&self) -> &'static str {
        "markdown"
    }

    fn display_name(&self) -> &'static str {
        "Markdown"
    }

    fn register(&self, cx: &mut PackContext<'_>) {
        cx.tree_sitter_language("markdown", |language| language);
        cx.tree_sitter_language("markdown_inline", |language| language);
    }
}
