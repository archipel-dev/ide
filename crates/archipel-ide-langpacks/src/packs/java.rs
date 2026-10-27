//! The Java pack: the reference language pack.
//!
//! Registers, for `java`:
//!
//! * the language, file type and parser (PLAT-LANG-001/020/030);
//! * the native resolve pass `java-resolve` on `HIGHLIGHTING_PASS` (PLAT-LANG-050): unresolved
//!   references, import fixes and the unused-import inspection from the Java indexes;
//! * the `@SuppressWarnings` suppressor on `INSPECTION_SUPPRESSOR` (PLAT-LANG-050);
//! * an `ImportFixProvider` for the `Method to Import` / `Class to Import` chooser;
//! * a `LanguageActivity` that keeps the indexes in line with the project model and the open
//!   buffers and adds the Java settings (PLAT-LANG-012);
//! * the jdtls language server (`LANGUAGE_SERVER_PROVIDER`);
//! * the JDK/jdtls/java-debug toolchain requirement.

use crate::{
    ActivityCx, ActivityOutcome, LanguageActivity, LanguagePack, PackContext, PackSetting,
    PackSettingDefault,
};
use archipel_ide_analysis::ChoiceRow;
use archipel_ide_analysis::ModCommand;
use archipel_ide_analysis::langext::{
    ChooserPopup, HIGHLIGHTING_PASS, INSPECTION_SUPPRESSOR, ImportFixProvider,
};
use archipel_ide_daemon::inspection::tier_a::native_suppressors;
use archipel_ide_java::diagnostics::{JavaResolvePass, JavaService, JavaSettings};
use archipel_ide_java::import_edit::ImportSettings;
use archipel_ide_language::{LanguageId, Order, Registration};
use archipel_ide_toolchains::providers::java_requirement;
use std::path::PathBuf;
use std::sync::Arc;

pub const ID: &str = "java";
pub const LANGUAGE: LanguageId = LanguageId("java");

/// Settings of the native Java pass (Settings | Editor | Inspections).
pub const SETTING_NATIVE: &str = "java.native.analysis";
pub const SETTING_UNUSED_IMPORT: &str = "java.native.unusedImport";
pub const SETTING_EXCLUDE: &str = "java.autoImport.exclude";

/// The Java pack. The [`JavaService`] (indexes, chooser state) lives as long as the pack.
pub struct JavaPack {
    service: Arc<JavaService>,
}

impl JavaPack {
    pub fn new() -> Self {
        Self {
            service: JavaService::new(),
        }
    }

    pub fn service(&self) -> &Arc<JavaService> {
        &self.service
    }
}

impl Default for JavaPack {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguagePack for JavaPack {
    fn id(&self) -> &'static str {
        ID
    }

    fn display_name(&self) -> &'static str {
        "Java"
    }

    fn import_command_prefixes(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn register(&self, cx: &mut PackContext<'_>) {
        cx.tree_sitter_language(ID, |language| language);
        cx.language_extension(
            HIGHLIGHTING_PASS,
            Registration::new("java-resolve", LANGUAGE).dumb_aware(true),
            Arc::new(JavaResolvePass {
                service: self.service.clone(),
            }),
        );
        for (id, language, suppressor) in native_suppressors() {
            if language == ID {
                cx.language_extension(
                    INSPECTION_SUPPRESSOR,
                    Registration::new(id, LANGUAGE)
                        .dumb_aware(true)
                        .order(Order::First),
                    suppressor,
                );
            }
        }
        let intelligence = Arc::new(JavaIntelligence {
            service: self.service.clone(),
        });
        cx.import_fix_provider(intelligence.clone());
        cx.activity(intelligence);
        if let Some(provider) = archipel_ide_lsp::builtin_provider("jdtls") {
            cx.language_server(provider);
        }
        cx.toolchain_requirement("java", java_requirement, None);
    }
}

/// The import fix provider and the per-project activity over the Java service.
struct JavaIntelligence {
    service: Arc<JavaService>,
}

impl ImportFixProvider for JavaIntelligence {
    fn id(&self) -> &'static str {
        "java"
    }

    fn languages(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn open_chooser(&self, title: String, rows: Vec<ChoiceRow>) {
        self.service.open_chooser(title, rows);
    }

    fn chooser(&self) -> Option<ChooserPopup> {
        self.service.chooser().map(|c| ChooserPopup {
            title: c.title,
            rows: c.rows,
        })
    }

    fn choose(&self, index: usize) -> Option<ModCommand> {
        self.service.choose(index)
    }

    fn close_chooser(&self) {
        self.service.close_chooser();
    }
}

impl LanguageActivity for JavaIntelligence {
    fn id(&self) -> &'static str {
        ID
    }

    fn settings(&self) -> Vec<PackSetting> {
        vec![
            PackSetting {
                key: SETTING_NATIVE,
                title: "Native Java analysis",
                group: "Java",
                description: Some(
                    "Cannot-resolve errors, import fixes and unused imports from the built-in Java indexes; the language server's duplicates are hidden",
                ),
                default: PackSettingDefault::Bool(true),
            },
            PackSetting {
                key: SETTING_UNUSED_IMPORT,
                title: "Unused import",
                group: "Java",
                description: None,
                default: PackSettingDefault::Bool(true),
            },
            PackSetting {
                key: SETTING_EXCLUDE,
                title: "Exclude from auto-import",
                group: "Java",
                description: Some(
                    "Comma separated class, member or package names (org.mockito.internal.*) never offered by import fixes",
                ),
                default: PackSettingDefault::Text(""),
            },
        ]
    }

    fn poll(&self, cx: &ActivityCx<'_>) -> ActivityOutcome {
        let native = cx.settings.bool(SETTING_NATIVE).unwrap_or(true);
        let unused = cx.settings.bool(SETTING_UNUSED_IMPORT).unwrap_or(true);
        let exclude = cx.settings.text(SETTING_EXCLUDE).unwrap_or_default();
        let mut wanted = JavaSettings {
            native,
            unused_imports: unused,
            imports: ImportSettings::default(),
        };
        wanted.imports.exclude = exclude
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let mut outcome = ActivityOutcome::default();
        if self.service.settings() != wanted {
            self.service.set_settings(wanted);
            outcome.native_wins = Some((ID, native));
            outcome.changed = true;
        }
        let Some(project) = cx.project else {
            return outcome;
        };
        outcome.changed |= self.service.sync(
            &project.root,
            project.model.clone(),
            project.model_loading,
            project.index_cache_dir.clone(),
        );
        if native {
            cx.documents.for_each(&mut |path, revision, text| {
                if path.extension().and_then(|e| e.to_str()) == Some("java") {
                    self.service.document_seen(path, revision, text);
                }
            });
        }
        outcome
    }

    fn paths_changed(&self, paths: &[PathBuf]) {
        self.service.paths_changed(paths);
    }

    fn status(&self) -> Option<String> {
        Some(format!("{:?}", self.service.status().state))
    }
}
