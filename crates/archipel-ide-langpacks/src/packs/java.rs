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
//! * a native `CompletionSource` (class names from the indexes with import on insertion, JAVA-CMP-018..021);
//! * the jdtls language server (`LANGUAGE_SERVER_PROVIDER`);
//! * the JDK/jdtls/java-debug toolchain requirement.

use crate::{
    ActivityCx, ActivityOutcome, LanguageActivity, LanguagePack, PackContext, PackSetting,
    PackSettingDefault,
};
use archipel_ide_analysis::ChoiceRow;
use archipel_ide_analysis::ModCommand;
use archipel_ide_analysis::langext::{
    ChooserPopup, CompletionSource, HIGHLIGHTING_PASS, INSPECTION_SUPPRESSOR, ImportFixProvider,
    SourceCompletion, SourceInsertion, SourceItem, SourceItemKind, SourceRequest,
};
use archipel_ide_daemon::inspection::tier_a::native_suppressors;
use archipel_ide_java::complete::{self, CompletionInput};
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
pub const SETTING_COMPLETION: &str = "java.native.completion";
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
        cx.completion_source(intelligence.clone());
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

impl CompletionSource for JavaIntelligence {
    fn id(&self) -> &'static str {
        ID
    }

    fn languages(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn complete(&self, request: &SourceRequest<'_>) -> Option<SourceCompletion> {
        let settings = self.service.settings();
        if !settings.completion {
            return None;
        }
        let project = self.service.project()?;
        let input = CompletionInput {
            project: &project,
            settings: &settings.imports,
            path: request.path,
            text: request.text,
            offset: request.offset,
            count: request.count,
        };
        match complete::complete(&input) {
            Ok(Some(c)) => Some(SourceCompletion {
                replace_start: c.prefix_start,
                exclusive: c.exclusive,
                note: c.note,
                items: c
                    .items
                    .iter()
                    .map(|i| SourceItem {
                        label: i.name.clone(),
                        filter_text: i.name.clone(),
                        detail: (!i.tail.is_empty()).then(|| i.tail.clone()),
                        kind: match i.kind {
                            archipel_ide_java::stub::ClassKind::Class => SourceItemKind::Class,
                            archipel_ide_java::stub::ClassKind::Interface => {
                                SourceItemKind::Interface
                            }
                            archipel_ide_java::stub::ClassKind::Enum => SourceItemKind::Enum,
                            archipel_ide_java::stub::ClassKind::Annotation => {
                                SourceItemKind::Annotation
                            }
                            archipel_ide_java::stub::ClassKind::Record => SourceItemKind::Record,
                        },
                        deprecated: i.deprecated,
                        key: i.fqn.clone(),
                        sort_text: i.sort.clone(),
                    })
                    .collect(),
            }),
            Ok(None) => None,
            // The indexes are still building: say so; the server answers meanwhile.
            Err(_) => {
                let c =
                    archipel_ide_java::complete::classify::classify(request.text, request.offset);
                (c.position != archipel_ide_java::complete::classify::Position::None).then(|| {
                    SourceCompletion {
                        replace_start: c.prefix_start,
                        items: Vec::new(),
                        exclusive: false,
                        note: Some("Indexing...".into()),
                    }
                })
            }
        }
    }

    fn insertion(&self, request: &SourceRequest<'_>, key: &str) -> Option<SourceInsertion> {
        let settings = self.service.settings();
        let project = self.service.project()?;
        let input = CompletionInput {
            project: &project,
            settings: &settings.imports,
            path: request.path,
            text: request.text,
            offset: request.offset,
            count: request.count,
        };
        let ins = complete::insertion(&input, key).ok()??;
        Some(SourceInsertion {
            text: ins.text,
            edits: ins.edits,
        })
    }

    fn auto_popup(&self, text_before_caret: &str) -> bool {
        self.service.settings().completion && complete::auto_popup_after(text_before_caret)
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
                key: SETTING_COMPLETION,
                title: "Native class-name completion",
                group: "Java",
                description: Some(
                    "Annotation types, classes after new and type positions from the built-in Java indexes, with the import added on insertion; the language server's duplicates are hidden",
                ),
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
        let completion = cx.settings.bool(SETTING_COMPLETION).unwrap_or(true);
        let mut wanted = JavaSettings {
            native,
            unused_imports: unused,
            completion,
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
            project.import_notes.clone(),
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

    fn notice(&self) -> Option<String> {
        let status = self.service.status();
        // An import in progress shows as busy elsewhere; a finished one with a problem stays.
        if status.classpath == archipel_ide_java::classpath::Completeness::Importing {
            return None;
        }
        status
            .classpath
            .describe()
            .map(|note| format!("Classpath incomplete: {note}"))
    }

    fn status(&self) -> Option<String> {
        let status = self.service.status();
        Some(match status.classpath.describe() {
            Some(note) => format!("{:?}; classpath incomplete: {note}", status.state),
            None => format!("{:?}", status.state),
        })
    }
}
