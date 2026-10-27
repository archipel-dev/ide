//! The built-in language packs.
//!
//! | Pack | Language(s) | Contributes |
//! |---|---|---|
//! | [`java`] | Java | native resolve pass, `@SuppressWarnings`, import fixes + chooser, project activity (indexes), jdtls, toolchain requirement |
//! | [`rust`] | Rust | `#[allow]` suppressor, rust-analyzer, toolchain requirement |
//! | [`kotlin`] | Kotlin | kotlin-lsp, toolchain requirement |
//! | [`toml`] | TOML | taplo |
//! | [`markdown`] | Markdown (+ inline) | language, file type, parser |
//! | [`bazel`] | Starlark | starlark server, bazelisk requirement |
//! | [`catalog`] | languages without a pack yet | their language servers |

pub mod bazel;
pub mod catalog;
pub mod java;
pub mod kotlin;
pub mod markdown;
pub mod rust;
pub mod toml;

use crate::LanguagePack;
use std::sync::Arc;

/// The packs installed at application start, in install order. Order matters where
/// extensions of different packs share a point (toolchain requirements are listed in this
/// order; Kotlin follows Java by `listed_after`).
pub fn builtin_packs() -> Vec<Arc<dyn LanguagePack>> {
    vec![
        Arc::new(java::JavaPack::new()),
        Arc::new(rust::RustPack),
        Arc::new(bazel::BazelPack),
        Arc::new(kotlin::KotlinPack),
        Arc::new(toml::TomlPack),
        Arc::new(markdown::MarkdownPack),
        Arc::new(catalog::CatalogPack),
    ]
}
