//! The Bazel pack: Starlark language, file type and parser, the starlark language server and
//! the bazelisk toolchain requirement of Bazel workspaces.

use crate::{LanguagePack, PackContext};
use archipel_ide_toolchains::providers::bazel_requirement;

pub struct BazelPack;

impl LanguagePack for BazelPack {
    fn id(&self) -> &'static str {
        "bazel"
    }

    fn display_name(&self) -> &'static str {
        "Bazel"
    }

    fn register(&self, cx: &mut PackContext<'_>) {
        cx.tree_sitter_language("starlark", |language| language);
        if let Some(provider) = archipel_ide_lsp::builtin_provider("starlark") {
            cx.language_server(provider);
        }
        cx.toolchain_requirement("bazel", bazel_requirement, None);
    }
}
