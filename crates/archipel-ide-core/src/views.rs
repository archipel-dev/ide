//! Read and write handles that the UI uses instead of reaching into `IdeState`'s fields.
//!
//! Phase 5 of the platform refactor: `archipel-ide-ui` renders and asks; it does not name the
//! fields of other subsystems (`crates/archipel-ide-ui/tests/no_subsystem_reads.rs` counts the
//! reads that are left). A handle is the seam where a subsystem becomes a service: today it
//! borrows what `IdeState` still owns, later the same call returns the service's guard and no
//! call site changes. Facts that menus and shortcuts need are `DataContext` keys instead
//! (see [`crate::ide_actions`]).

use crate::{IdeState, tabs::EditorTabs};
use archipel_ide_project::Project;

impl IdeState {
    // Status bar.

    /// The status bar message.
    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn set_status(&mut self, message: String) {
        self.status = message;
    }

    pub fn clear_status(&mut self) {
        self.status.clear();
    }

    // Project.

    pub fn has_project(&self) -> bool {
        self.project.is_some()
    }

    /// The file model of the open project.
    pub fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    // Editors.

    /// The open documents, in tab order.
    pub fn documents(&self) -> &[crate::Document] {
        &self.documents
    }

    pub fn documents_mut(&mut self) -> &mut [crate::Document] {
        &mut self.documents
    }

    pub fn document(&self, index: usize) -> Option<&crate::Document> {
        self.documents.get(index)
    }

    pub fn document_mut(&mut self, index: usize) -> Option<&mut crate::Document> {
        self.documents.get_mut(index)
    }

    /// Index of the document in the selected editor tab.
    pub fn active_index(&self) -> Option<usize> {
        self.active_document
    }

    pub fn set_active_index(&mut self, index: Option<usize>) {
        self.active_document = index;
    }

    /// The document in the selected editor tab.
    pub fn active_doc(&self) -> Option<&crate::Document> {
        self.documents.get(self.active_document?)
    }

    pub fn active_doc_mut(&mut self) -> Option<&mut crate::Document> {
        let index = self.active_document?;
        self.documents.get_mut(index)
    }

    /// The document at `index` for editing, with the inline rename session (if it is renaming
    /// in this document): the two borrows are disjoint, so the editor can draw both.
    pub fn document_for_editing(
        &mut self,
        index: usize,
    ) -> Option<(&mut crate::Document, Option<&crate::RenameSession>)> {
        let document = self.documents.get_mut(index)?;
        let rename = self
            .refactor
            .rename
            .as_ref()
            .filter(|session| session.path == document.path && !session.dialog);
        Some((document, rename))
    }

    /// Pinned tabs and the closed-tab history.
    pub fn editor_tabs(&self) -> &EditorTabs {
        &self.tabs
    }
}

/// A shared and an exclusive handle for each subsystem the UI still reads whole.
macro_rules! handles {
    ($($(#[$doc:meta])* $name:ident, $name_mut:ident: $ty:ty;)*) => {
        impl IdeState {
            $(
                $(#[$doc])*
                pub fn $name(&self) -> &$ty {
                    &self.$name
                }

                $(#[$doc])*
                pub fn $name_mut(&mut self) -> &mut $ty {
                    &mut self.$name
                }
            )*
        }
    };
}

handles! {
    /// Android project, devices, logcat, SDK manager and layout editor.
    android, android_mut: crate::AndroidState;
    /// Agents mode: sessions, grid, review.
    agents, agents_mut: crate::agents::AgentsState;
    /// The plugin registry and the contributions of plugins.
    plugin_host, plugin_host_mut: crate::PluginHostState;
    /// The AI assistant.
    ai, ai_mut: crate::AiState;
    /// Tool window layout.
    tool_windows, tool_windows_mut: crate::ToolWindowLayout;
    /// The native analysis world.
    analysis, analysis_mut: crate::AnalysisServices;
    /// Navigation history and requests.
    navigation, navigation_mut: crate::NavigationState;
    /// Quick fixes and intentions at the caret.
    code_actions, code_actions_mut: crate::CodeActionState;
    /// Editor split tree.
    splits, splits_mut: archipel_ide_split::SplitTree<std::path::PathBuf>;
    /// Language servers.
    lsp, lsp_mut: archipel_ide_lsp::LspManager;
    /// Project tool window options, marks and clipboard.
    project_view, project_view_mut: crate::project_view::ProjectView;
    /// The completion popup, while open.
    completion, completion_mut: Option<crate::CompletionSession>;
    /// Quick documentation.
    docs, docs_mut: crate::DocsState;
    /// Rename and the other refactorings.
    refactor, refactor_mut: crate::RefactorState;
    /// Type and call hierarchy.
    hierarchy, hierarchy_mut: crate::HierarchyState;
    /// Parameter info.
    signature, signature_mut: crate::SignatureState;
    /// Live template session.
    snippet, snippet_mut: Option<crate::snippet_session::SnippetSession>;
    /// Project-wide problems.
    problems, problems_mut: crate::ProblemIndex;
    /// The diff viewer tab, if open.
    diff, diff_mut: Option<crate::vcs::DiffTab>;
}

/// Read-only handles to values the UI shows.
macro_rules! getters {
    ($($(#[$doc:meta])* $name:ident: $ty:ty;)*) => {
        impl IdeState {
            $(
                $(#[$doc])*
                pub fn $name(&self) -> &$ty {
                    &self.$name
                }
            )*
        }
    };
}

getters! {
    /// Gradle model from the last sync.
    gradle_model: Option<archipel_ide_gradle::GradleModel>;
    gradle_error: Option<String>;
    /// Bazel targets from the last sync.
    bazel_targets: Option<archipel_ide_bazel::BazelTargets>;
    bazel_error: Option<String>;
    /// Build system of the open project.
    detected_build: Option<crate::DetectedBuild>;
    /// Files of the open project.
    files: Vec<archipel_ide_project::FileEntry>;
    /// Most recently used files, newest first.
    recent_files: Vec<std::path::PathBuf>;
    /// Project-relative directories expanded in the Project tool window.
    expanded_dirs: std::collections::BTreeSet<std::path::PathBuf>;
    /// Unseen crash report of a previous session.
    crash_report: Option<std::path::PathBuf>;
}

impl IdeState {
    pub fn workbench_mode(&self) -> crate::agents::WorkbenchMode {
        self.workbench_mode
    }

    pub fn about_open(&self) -> bool {
        self.about_open
    }

    pub fn first_run_open(&self) -> bool {
        self.first_run_open
    }

    /// Build scripts changed on disk since the last Gradle sync.
    pub fn gradle_changes_pending(&self) -> bool {
        self.gradle_changes_pending
    }

    /// Whether the diff viewer tab is the selected editor tab (even when no diff is open).
    pub fn diff_active(&self) -> bool {
        self.diff_active
    }

    pub fn set_diff_active(&mut self, active: bool) {
        self.diff_active = active;
    }

    /// The settings page the UI should open, once.
    pub fn take_settings_request(&mut self) -> Option<String> {
        self.settings_request.take()
    }
}
