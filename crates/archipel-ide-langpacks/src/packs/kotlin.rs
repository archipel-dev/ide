//! The Kotlin pack: language, file type, parser, language server and toolchain requirement.
//! Kotlin has no native intelligence yet (tier G0): the language server answers.

use crate::{LanguagePack, PackContext};
use archipel_ide_toolchains::providers::kotlin_requirement;

pub struct KotlinPack;

impl LanguagePack for KotlinPack {
    fn id(&self) -> &'static str {
        "kotlin"
    }

    fn display_name(&self) -> &'static str {
        "Kotlin"
    }

    fn register(&self, cx: &mut PackContext<'_>) {
        cx.tree_sitter_language("kotlin", |language| language);
        if let Some(provider) = archipel_ide_lsp::builtin_provider("kotlin") {
            cx.language_server(provider);
        }
        // The Kotlin server is listed after the JDK components of a Java build.
        cx.toolchain_requirement("kotlin", kotlin_requirement, Some("Java"));
    }
}
