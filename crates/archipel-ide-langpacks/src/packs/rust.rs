//! The Rust pack (PLAT-LANG-001/020/030 language, file type and parser; PLAT-LANG-050
//! suppressor; language server and toolchain requirement).

use crate::{LanguagePack, PackContext};
use archipel_ide_analysis::langext::INSPECTION_SUPPRESSOR;
use archipel_ide_daemon::inspection::tier_a::native_suppressors;
use archipel_ide_language::{LanguageId, Registration};
use archipel_ide_toolchains::providers::rust_requirement;

pub struct RustPack;

impl LanguagePack for RustPack {
    fn id(&self) -> &'static str {
        "rust"
    }

    fn display_name(&self) -> &'static str {
        "Rust"
    }

    fn register(&self, cx: &mut PackContext<'_>) {
        cx.tree_sitter_language("rust", |language| language);
        for (id, language, suppressor) in native_suppressors() {
            if language == "rust" {
                cx.language_extension(
                    INSPECTION_SUPPRESSOR,
                    Registration::new(id, LanguageId(language))
                        .dumb_aware(true)
                        .order(archipel_ide_language::Order::First),
                    suppressor,
                );
            }
        }
        if let Some(provider) = archipel_ide_lsp::builtin_provider("rust-analyzer") {
            cx.language_server(provider);
        }
        cx.toolchain_requirement("rust", rust_requirement, None);
    }
}
