//! The TOML pack: language, file type, parser and the taplo language server.

use crate::{LanguagePack, PackContext};

pub struct TomlPack;

impl LanguagePack for TomlPack {
    fn id(&self) -> &'static str {
        "toml"
    }

    fn display_name(&self) -> &'static str {
        "TOML"
    }

    fn register(&self, cx: &mut PackContext<'_>) {
        cx.tree_sitter_language("toml", |language| language);
        if let Some(provider) = archipel_ide_lsp::builtin_provider("taplo") {
            cx.language_server(provider);
        }
    }
}
