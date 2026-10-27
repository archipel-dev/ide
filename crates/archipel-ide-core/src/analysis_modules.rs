//! The analysis packages plugged into the app: one line per package or
//! language crate. Each later package fills in its own entry (services into
//! the analysis registry, commands into the command registry) without
//! touching `IdeState` or `lib.rs` (spec Part 4 "integration seam").

use crate::IdeState;
use archipel_ide_analysis::ServiceRegistry;
use archipel_ide_command::CommandRegistry;

/// One analysis package.
pub(crate) struct AnalysisModule {
    /// Package id (for diagnostics and `analysis.status`).
    pub id: &'static str,
    /// Registers language services (called once per analysis host).
    pub services: fn(&mut ServiceRegistry),
    /// Registers the package's commands (its `<package>_commands.rs`).
    pub commands: fn(&mut CommandRegistry<IdeState>),
}

fn none(_: &mut ServiceRegistry) {}

fn no_commands(_: &mut CommandRegistry<IdeState>) {}

/// Registration order is service order within a slot (see `Order`).
pub(crate) const MODULES: &[AnalysisModule] = &[
    // WP1 analysis core: generic services come from `default_registry`.
    AnalysisModule {
        id: "analysis-core",
        services: none,
        commands: crate::analysis_host::register,
    },
    // WP2 index, dumb mode, TODO, local history (`archipel-ide-index`).
    AnalysisModule {
        id: "index",
        services: crate::indexing_commands::register_services,
        commands: crate::indexing_commands::register_all,
    },
    // WP3 daemon, inspections, intentions (`archipel-ide-daemon`).
    AnalysisModule {
        id: "daemon",
        services: crate::inspection_commands::daemon_services,
        commands: crate::inspection_commands::register,
    },
    // WP4 structural features and typed editing (`archipel-ide-language` queries).
    AnalysisModule {
        id: "structure",
        services: none,
        commands: no_commands,
    },
    // WP5 resolve, usages, navigation (`archipel-ide-resolve`).
    AnalysisModule {
        id: "resolve",
        services: none,
        commands: no_commands,
    },
    // WP6 completion, formatting, templates.
    AnalysisModule {
        id: "completion",
        services: none,
        commands: no_commands,
    },
    AnalysisModule {
        id: "format",
        services: none,
        commands: no_commands,
    },
    AnalysisModule {
        id: "templates",
        services: none,
        commands: no_commands,
    },
    // WP7 refactoring framework (`archipel-ide-refactor`).
    AnalysisModule {
        id: "refactor",
        services: none,
        commands: no_commands,
    },
    // WP8 LSP provider bridge (`archipel-ide-lsp-provider`), `Order::Last`.
    AnalysisModule {
        id: "lsp-provider",
        services: none,
        commands: no_commands,
    },
    // WP9 language tiers (`archipel-ide-lang-*`).
    AnalysisModule {
        id: "lang-java",
        services: crate::java_commands::java_services,
        commands: crate::java_commands::register_commands,
    },
    AnalysisModule {
        id: "lang-rust",
        services: none,
        commands: no_commands,
    },
    AnalysisModule {
        id: "lang-data",
        services: none,
        commands: no_commands,
    },
    AnalysisModule {
        id: "lang-kotlin",
        services: none,
        commands: no_commands,
    },
    AnalysisModule {
        id: "lang-web",
        services: none,
        commands: no_commands,
    },
];

/// The service registry of the app: generic services, every language's
/// `services` hook, then every module.
pub(crate) fn service_registry() -> ServiceRegistry {
    let mut registry = archipel_ide_analysis::default_registry();
    for module in MODULES {
        (module.services)(&mut registry);
    }
    registry
}

pub(crate) fn register_commands(registry: &mut CommandRegistry<IdeState>) {
    for module in MODULES {
        (module.commands)(registry);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn module_ids_are_unique() {
        let mut ids: Vec<_> = super::MODULES.iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), super::MODULES.len());
    }
}
