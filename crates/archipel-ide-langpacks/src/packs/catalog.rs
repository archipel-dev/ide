//! Language servers of the languages that have no pack of their own yet (Python, Go,
//! TypeScript/JavaScript, C/C++, XML, YAML, JSON). Their grammars stay in the static table of
//! `archipel-ide-language`; the next step is one pack per language like [`super::rust`].

use crate::{LanguagePack, PackContext};

pub struct CatalogPack;

/// Server ids of the catalog (`archipel_ide_lsp::builtin_provider`).
pub const SERVERS: &[&str] = &[
    "pyright",
    "gopls",
    "typescript",
    "clangd",
    "lemminx",
    "yaml",
    "json",
];

impl LanguagePack for CatalogPack {
    fn id(&self) -> &'static str {
        "catalog"
    }

    fn display_name(&self) -> &'static str {
        "Language servers without a pack"
    }

    fn register(&self, cx: &mut PackContext<'_>) {
        for id in SERVERS {
            if let Some(provider) = archipel_ide_lsp::builtin_provider(id) {
                cx.language_server(provider);
            }
        }
    }
}
