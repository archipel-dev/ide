//! Native-vs-LSP coexistence contracts (implemented by WP8's
//! `archipel-ide-lsp-provider`). Providers register adapters for the "any"
//! language with `Order::Last`, so native services win on conflict; the
//! per-language/per-feature [`ProviderMode`] decides who answers and
//! [`Merger`]s combine results.

use crate::target::Confidence;
use archipel_ide_language::{CompletenessTier, LanguageId, ServiceRegistry};
use std::collections::HashMap;

/// `analysis.<lang>.provider` (per feature).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProviderMode {
    /// Native where the language's tier covers the feature, LSP otherwise.
    #[default]
    Auto,
    Native,
    Lsp,
    /// Both, merged; differences are logged (differential testing).
    Both,
}

/// A feature family the mode is chosen for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Feature {
    Highlights,
    Resolve,
    Implementations,
    Usages,
    Rename,
    Completion,
    QuickFixes,
    Documentation,
    Structure,
    InlayHints,
    Formatting,
}

impl Feature {
    /// Lowest tier at which native answers are preferred under `Auto`.
    pub fn native_tier(self) -> CompletenessTier {
        match self {
            Feature::Structure | Feature::Highlights | Feature::Formatting => CompletenessTier::G0,
            Feature::Completion | Feature::QuickFixes => CompletenessTier::G1,
            Feature::Resolve | Feature::Usages | Feature::Rename | Feature::Documentation => {
                CompletenessTier::G2
            }
            Feature::Implementations | Feature::InlayHints => CompletenessTier::G3,
        }
    }
}

/// Which engines answer a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Engines {
    pub native: bool,
    pub lsp: bool,
}

/// Mode settings per (language, feature) with a per-language default.
#[derive(Debug, Clone, Default)]
pub struct ProviderSettings {
    pub modes: HashMap<(LanguageId, Option<Feature>), ProviderMode>,
}

impl ProviderSettings {
    pub fn mode(&self, language: LanguageId, feature: Feature) -> ProviderMode {
        self.modes
            .get(&(language, Some(feature)))
            .or_else(|| self.modes.get(&(language, None)))
            .copied()
            .unwrap_or_default()
    }

    /// Who answers `feature` for a language at `tier` with a server present.
    pub fn engines(
        &self,
        language: LanguageId,
        tier: CompletenessTier,
        feature: Feature,
        server: bool,
    ) -> Engines {
        let native_ok = tier >= feature.native_tier();
        match self.mode(language, feature) {
            ProviderMode::Native => Engines {
                native: true,
                lsp: false,
            },
            ProviderMode::Lsp => Engines {
                native: !server,
                lsp: server,
            },
            ProviderMode::Both => Engines {
                native: true,
                lsp: server,
            },
            ProviderMode::Auto => Engines {
                native: native_ok || !server,
                lsp: server && !native_ok,
            },
        }
    }
}

/// A bundle of adapters registering itself (the LSP provider, a language
/// crate's native services).
pub trait AnalysisProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn register(&self, registry: &mut ServiceRegistry);
}

/// Combines native and LSP results of one feature.
pub trait Merger<T>: Send + Sync {
    fn merge(&self, native: Vec<T>, lsp: Vec<T>, mode: ProviderMode) -> Vec<T>;
}

/// Union with dedupe by key; on duplicates the item from the preferred side
/// stays (native when `prefer_native`).
pub fn merge_dedupe<T, K: PartialEq>(
    native: Vec<T>,
    lsp: Vec<T>,
    key: impl Fn(&T) -> K,
    prefer_native: bool,
) -> Vec<T> {
    let (first, second) = if prefer_native {
        (native, lsp)
    } else {
        (lsp, native)
    };
    let mut out: Vec<T> = Vec::with_capacity(first.len() + second.len());
    for item in first.into_iter().chain(second) {
        let k = key(&item);
        if !out.iter().any(|existing| key(existing) == k) {
            out.push(item);
        }
    }
    out
}

/// Whether native should also ask the LSP (resolve policy: `Exact` is final).
pub fn needs_fallback(native: Option<Confidence>) -> bool {
    !matches!(native, Some(Confidence::Exact))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_mode_follows_the_tier_and_overrides_win() {
        let rust = LanguageId("rust");
        let mut settings = ProviderSettings::default();
        let auto = settings.engines(rust, CompletenessTier::G0, Feature::Resolve, true);
        assert_eq!(
            auto,
            Engines {
                native: false,
                lsp: true
            }
        );
        let structure = settings.engines(rust, CompletenessTier::G0, Feature::Structure, true);
        assert_eq!(
            structure,
            Engines {
                native: true,
                lsp: false
            }
        );
        let no_server = settings.engines(rust, CompletenessTier::G0, Feature::Resolve, false);
        assert!(no_server.native && !no_server.lsp);
        settings.modes.insert((rust, None), ProviderMode::Both);
        settings
            .modes
            .insert((rust, Some(Feature::Rename)), ProviderMode::Lsp);
        assert_eq!(settings.mode(rust, Feature::Usages), ProviderMode::Both);
        assert_eq!(settings.mode(rust, Feature::Rename), ProviderMode::Lsp);
    }

    #[test]
    fn merge_dedupes_with_preference() {
        let merged = merge_dedupe(
            vec![(1, "n1"), (2, "n2")],
            vec![(2, "l2"), (3, "l3")],
            |x| x.0,
            true,
        );
        assert_eq!(merged, vec![(1, "n1"), (2, "n2"), (3, "l3")]);
        let merged = merge_dedupe(vec![(2, "n2")], vec![(2, "l2")], |x| x.0, false);
        assert_eq!(merged, vec![(2, "l2")]);
        assert!(
            needs_fallback(Some(Confidence::Heuristic)) && !needs_fallback(Some(Confidence::Exact))
        );
    }
}
