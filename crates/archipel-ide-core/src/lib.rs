use archipel_ide_command::{Command, CommandError, CommandRegistry, CommandSpec};
use archipel_ide_git::RepositoryStatus;
use archipel_ide_language::{LanguageAnalysis, analyze_lazy};
use archipel_ide_lsp::{LspDiagnostic, LspManager};
use archipel_ide_project::{FileEntry, Project};
use archipel_ide_search::SearchEvent;
pub use archipel_ide_search::TextMatch;
use archipel_ide_terminal::TerminalSession;
use archipel_ide_text::{EditJournal, Selection, SelectionSet, TextDocument};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub mod agent_events;
pub mod agents;
pub mod agents_mock;
mod ai_actions;
mod analysis_host;
mod analysis_modules;
mod automation_commands;
mod history_commands;
pub mod ide_actions;
mod indexing_commands;
mod todo_commands;
pub use analysis_host::AnalysisServices;
mod inspection_commands;
mod language_packs;
/// Alt+Enter model types for the UI.
pub mod intentions {
    pub use archipel_ide_analysis::inspect::Level;
    pub use archipel_ide_analysis::intention::IntentionGroup;
    pub use archipel_ide_daemon::intention::{AlertAction, AlertSet, Bulb};
}
pub use inspection_commands::{
    INSPECTIONS_PAGE, InspectionRow, SETTING_DELAY, SETTING_ENABLED, SETTING_PROFILE, level_id,
};
mod ai_agent;
mod ai_commands;
mod ai_inline;
mod ai_terminal;
mod ai_vcs;
mod android_commands;
mod android_deploy;
mod android_devices;
mod android_logcat;
mod android_manifest;
mod android_resources;
mod android_sdk_repo;
mod android_tools;
mod android_variants;
mod annotations;
mod bazel_commands;
mod branch_commands;
mod build_commands;
pub mod build_info;
mod build_window;
mod code_actions;
mod code_vision;
mod completion;
mod completion_native;
mod completion_stats;
mod debug_commands;
mod diagnostic_commands;
pub mod diagnostics;
mod disk_sync;
mod editor_commands;
pub mod editor_input;
mod external_diagnostics;
pub mod find_in_files;
pub mod find_session;
pub mod first_run;
mod folds;
mod git_log;
pub mod goto_line;
pub mod goto_rank;
mod gradle_commands;
mod gutter_commands;
mod inline_values;
mod java_debug;
pub mod keymap_commands;
mod line_changes;
mod lsp_commands;
mod markdown_commands;
pub mod name_match;
mod navigation;
mod new_class;
pub mod platform;
mod plugin_host;
mod problems;
pub mod quick_nav;
pub mod recent_popup;
pub mod search_popup;
pub mod services;
pub use new_class::{FileTemplate, JavaKind, KotlinKind, file_templates};
mod generate;
mod hierarchy;
mod project_commands;
pub mod project_model;
pub mod project_view;
mod quick_doc;
mod refactor;
mod remote_commands;
mod search_replace;
pub mod spring_commands;
mod test_nav;
mod welcome_commands;
pub use saving::{SaveOutcome, SavePoint, texts as saving_texts};
pub use services::CloneJob;
pub use services::{DocumentSaveListener, DocumentSaving, SaveFailureReport, SavingVetoer};
pub use welcome_commands::{JdkChoice, jdk_major, repository_name};
mod live_templates;
mod run_config_commands;
mod saving;
mod search_scopes;
mod settings_commands;
mod signature_help;
mod snippet_session;
pub mod splits;
pub mod tabs;
mod terminal_commands;
mod terminal_tabs;
mod toolchain_commands;
mod toolchain_lsp;
mod toolwindow;
mod update_commands;
mod views;
pub use android_commands::AndroidState;
pub use android_deploy::{DeployPhase, DeployState};
pub use android_devices::DeviceHub;
pub use android_logcat::LogcatState;
pub use android_tools::{LayoutEditorState, LayoutMode};
pub use disk_sync::{DiskConflict, Waker, content_hash};
pub use services::Settings;
mod vcs;
pub use archipel_ide_runnables::RunnableKind;
pub use archipel_ide_search::SearchOptions;
pub use build_commands::{BuildSystems, DetectedBuild};
pub use completion::CompletionSession;
pub use gutter_commands::GutterMark;

/// Where Find in Files looks (the tabs of the Find in Files popup).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchScopeKind {
    #[default]
    Project,
    /// The Gradle module that contains the active file.
    Module,
    /// `project_view.search_scope`.
    Directory,
    OpenFiles,
    ProductionFiles,
    TestFiles,
    /// A user-defined scope: `IdeState::search_custom_scope`.
    Custom,
}

impl SearchScopeKind {
    pub fn id(self) -> &'static str {
        match self {
            SearchScopeKind::Project => "project",
            SearchScopeKind::Module => "module",
            SearchScopeKind::Directory => "directory",
            SearchScopeKind::OpenFiles => "openFiles",
            SearchScopeKind::ProductionFiles => "production",
            SearchScopeKind::TestFiles => "tests",
            SearchScopeKind::Custom => "custom",
        }
    }

    pub fn parse(id: &str) -> Option<SearchScopeKind> {
        [
            SearchScopeKind::Project,
            SearchScopeKind::Module,
            SearchScopeKind::Directory,
            SearchScopeKind::OpenFiles,
            SearchScopeKind::ProductionFiles,
            SearchScopeKind::TestFiles,
            SearchScopeKind::Custom,
        ]
        .into_iter()
        .find(|kind| kind.id() == id)
    }
}
pub use plugin_host::{
    PluginHostState, PluginStatusWidget, PluginTab, plugin_dirs, plugin_registry_dirs,
    sync_plugin_commands, user_plugin_dir,
};
mod workspace_edit;
pub use ai_agent::{
    AgentPhase, AgentRun, AgentStep, StepKind as AgentStepKind, StepStatus as AgentStepStatus,
};
pub use ai_commands::{AiState, ApplyPreview, CHAT_FILE as AI_CHAT_FILE, MOCK_ENV as AI_MOCK_ENV};
pub use ai_inline::InlineSuggestion;
pub use ai_terminal::{ShellSuggestion, ShellSuggestionState};
pub use build_window::{BuildSnapshot, BuildTab, BuildWindowState};
pub use code_actions::{CodeActionItem, CodeActionList, CodeActionState};
pub use code_vision::{CodeLens, InlayLabel};
pub use completion_stats::CompletionStats;
pub use debug_commands::{BREAKPOINTS_FILE, DEBUG_STATE_FILE, ExecutionPoint};
pub use generate::{GenerateKind, GenerateState};
pub use hierarchy::{
    Hierarchy, HierarchyDirection, HierarchyKind, HierarchyNode, HierarchyRow, HierarchyState,
};
pub use inline_values::InlineValue;
pub use live_templates::context_labels;
pub use navigation::{NavigationItem, NavigationKind, NavigationList, NavigationState, Place};
pub use problems::{FileProblems, Problem, ProblemCounts, ProblemIndex, Severity};
pub use quick_doc::{DocsState, QuickDoc};
pub use refactor::{RefactorState, Refactoring, RenameSession};
pub use services::{ToolchainBanner, ToolchainState};
pub use signature_help::{ParameterInfo, SignatureLine, SignatureState};
pub use snippet_session::SnippetSession;
pub use todo_commands::{TodoGroup, TodoRow, TodoScope, TodoState, TodoView};
mod worktree_commands;
pub use branch_commands::abort_label;
pub use folds::{CollapsedFold, FoldDefaults, FoldRegion, FoldState};
pub use git_log::{DateRange, GitLogState};
pub use line_changes::hunk_at_line;
pub use toolwindow::{
    Anchor, BOTTOM_DEFAULT_HEIGHT, BOTTOM_MIN_HEIGHT, EDITOR_MIN_HEIGHT, ToolWindow,
    ToolWindowLayout,
};
pub use update_commands::{
    CURRENT_VERSION, UpdateState, installable_executable as update_installable,
};
pub use vcs::{DiffHighlight, DiffTab, DiffViewer, RepaintHook};
pub use workspace_edit::EditSummary;

#[derive(Debug, Clone)]
pub struct Document {
    pub path: PathBuf,
    pub text: TextDocument,
    pub analysis: LanguageAnalysis,
    pub selections: SelectionSet,
    /// Hash of the content last loaded from or saved to disk.
    pub disk_hash: u64,
    /// Unresolved outside change; saving is refused while set.
    pub conflict: Option<DiskConflict>,
    /// Selections before each Extend Selection step, for Shrink Selection.
    pub selection_history: Vec<Vec<Selection>>,
    /// Collapsed code-folding regions.
    pub folds: FoldState,
    /// Edits not yet seen by the analysis host (see `poll_analysis`).
    pub(crate) analysis_sync: analysis_host::PendingAnalysisSync,
    /// Whether the document belongs to a file on disk (PLAT-SAVE-003).
    pub backing: DocumentBacking,
    /// Lines that kept trailing spaces because a caret was on them at the last save; the next
    /// save strips them once the caret left (PLAT-SAVE-091).
    pub(crate) kept_trailing: Vec<String>,
    /// Saves this document even when its text equals the disk text (PLAT-SAVE-075).
    pub force_save: bool,
    /// Modes and memory of the input layer (overwrite, sticky selection, goal column, tab-out).
    pub input: editor_input::InputState,
    /// The Code Folding settings at the time the file was opened: they decide which regions the
    /// document has (the one-line method form) and which start collapsed (ED-FOLD-020).
    pub fold_defaults: folds::FoldDefaults,
}

/// What a document is backed by (PLAT-SAVE-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentBacking {
    /// A physical file: tracked, saved.
    #[default]
    File,
    /// A copy, fragment or preview used only for analysis: never unsaved, saving is a no-op.
    Virtual,
    /// A virtual document whose file opted into tracking.
    TrackedVirtual,
}

impl Document {
    /// Whether the document takes part in the unsaved set (PLAT-SAVE-003).
    pub fn is_tracked(&self) -> bool {
        !matches!(self.backing, DocumentBacking::Virtual)
    }

    /// Unsaved: tracked and its text differs from the last saved state (PLAT-SAVE-001).
    pub fn is_unsaved(&self) -> bool {
        self.is_tracked() && self.text.is_dirty()
    }
}

impl Document {
    /// Creates a document and its initial language analysis.
    pub fn new(path: PathBuf, text: &str) -> Self {
        let mut document = Self {
            analysis: analyze_lazy(&path, text),
            text: TextDocument::new(text),
            path,
            selections: SelectionSet::default(),
            disk_hash: content_hash(text),
            conflict: None,
            selection_history: Vec::new(),
            folds: FoldState::default(),
            analysis_sync: Default::default(),
            backing: DocumentBacking::File,
            kept_trailing: Vec::new(),
            force_save: false,
            input: Default::default(),
            fold_defaults: Default::default(),
        };
        // The analysis already reflects `text`.
        document.text.take_edits();
        document
    }

    /// Brings the language analysis up to date with the buffer: journaled
    /// edits are applied incrementally; otherwise the file is reanalyzed.
    pub fn refresh_analysis(&mut self, source: &str) {
        match self.text.take_edits() {
            EditJournal::Edits(edits) => {
                for edit in &edits {
                    self.folds.apply_edit(edit);
                }
                self.analysis.apply_edits(&self.path, source, &edits);
                self.analysis_sync
                    .record(self.text.revision(), Some(&edits));
            }
            EditJournal::Reset => {
                self.folds.clear();
                self.analysis = analyze_lazy(&self.path, source);
                self.analysis_sync.record(self.text.revision(), None);
            }
        }
    }
}

#[derive(Default)]
pub struct IdeState {
    /// The platform kernel: the [`archipel_ide_platform::Application`], the open
    /// [`archipel_ide_platform::Project`] and their services (toolchains, settings,
    /// notifications, Git status). Use the accessors in [`platform`].
    pub(crate) platform: platform::Platform,
    pub project: Option<Project>,
    pub files: Vec<FileEntry>,
    /// Bumped whenever [`IdeState::files`] is replaced; keys the tree-row cache.
    pub(crate) files_generation: u64,
    /// Bumped when the Gradle model changes; keys the tree-row cache.
    pub(crate) model_generation: u64,
    pub(crate) tree_cache: Option<project_view::TreeCache>,
    pub(crate) problems_cache:
        std::collections::HashMap<std::path::PathBuf, problems::ActiveProblemsCache>,
    pub documents: Vec<Document>,
    pub active_document: Option<usize>,
    /// The native analysis world (host, snapshots, services); synced with
    /// `documents` by [`IdeState::poll_analysis`].
    pub analysis: AnalysisServices,
    /// Markdown editor layout per file (Editor / Split / Preview).
    pub markdown_layouts:
        std::collections::HashMap<std::path::PathBuf, archipel_ide_markdown::Layout>,
    /// The loaded plugin code. Instances are not `Send`, so they stay on the UI thread here;
    /// the registry and the plugin views are the `PluginHostService`.
    pub(crate) plugin_runtime: archipel_ide_plugins::PluginRuntime,
    /// Pinned tabs, closed-tab history and the tab limit.
    pub tabs: tabs::EditorTabs,
    /// Editor split tree: which tabs live in which group (PLATFORM-UI-004).
    pub splits: archipel_ide_split::SplitTree<PathBuf>,
    /// The active tab as of the last `sync_splits`; a different one means
    /// something outside the split model activated a tab.
    splits_synced: Option<PathBuf>,
    /// Code vision and inlay hints per document.
    code_vision: code_vision::CodeVisionState,
    /// The diff viewer tab, if open.
    pub diff: Option<DiffTab>,
    /// Whether the diff tab (rather than a document) is the selected editor tab.
    pub diff_active: bool,
    /// Lets background services wake the UI.
    pub repaint: RepaintHook,
    /// Language servers of the open project, started lazily per language.
    pub lsp: LspManager,
    /// Project-wide problems, refreshed by [`IdeState::refresh_problems`].
    pub problems: ProblemIndex,
    /// The code completion popup, while open.
    pub completion: Option<CompletionSession>,
    /// What the user picked before, to rank it first next time.
    pub completion_stats: CompletionStats,
    /// Tab stops of an inserted snippet, while the user fills them in.
    pub snippet: Option<SnippetSession>,
    pub build_systems: BuildSystems,
    /// Build system of the open project, detected on open (`build.detect` refreshes it).
    pub detected_build: Option<DetectedBuild>,
    /// Navigation history and pending/finished navigation requests.
    pub navigation: NavigationState,
    /// Quick fixes and intentions at the caret (Alt+Enter).
    pub code_actions: CodeActionState,
    /// Rename and the other server-backed refactorings.
    pub refactor: RefactorState,
    /// The Generate popup's pending requests.
    pub generate: GenerateState,
    /// Type / Call hierarchy tree and Go to Super Method.
    pub hierarchy: HierarchyState,
    /// Quick documentation popup and completion documentation panel.
    pub docs: DocsState,
    /// Parameter info popup.
    pub signature: SignatureState,
    /// Gradle model from the last sync (cached in `.archipel/gradle/model.json`).
    pub gradle_model: Option<archipel_ide_gradle::GradleModel>,
    pub gradle_error: Option<String>,
    /// Build scripts changed on disk since the last sync ("Load Gradle Changes").
    pub gradle_changes_pending: bool,
    /// Bazel targets from the last sync (cached in `.archipel/bazel/targets.json`).
    pub bazel_targets: Option<archipel_ide_bazel::BazelTargets>,
    pub bazel_error: Option<String>,
    pub tool_windows: ToolWindowLayout,
    /// Code or Agents arrangement of the window (documents and tabs are shared).
    pub workbench_mode: agents::WorkbenchMode,
    /// Project-relative directories expanded in the Project tool window.
    pub expanded_dirs: BTreeSet<PathBuf>,
    /// Project view options, directory marks and the file clipboard.
    pub project_view: project_view::ProjectView,
    /// Most recently used files, newest first (Recent Files, ⌘E).
    pub recent_files: Vec<PathBuf>,
    /// Whether Help → About is showing (`app.about`).
    pub about_open: bool,
    /// Unseen crash report from a previous session (`app.dismissCrashReport`).
    pub crash_report: Option<PathBuf>,
    /// Whether the first-run "Customize Archipel IDE" wizard is showing.
    pub first_run_open: bool,
    /// Config directory the first-run marker is written to.
    first_run_dir: Option<PathBuf>,
    /// Settings page the UI should open (set by `settings.open`, taken by the UI).
    pub settings_request: Option<String>,
    pub ai: AiState,
    pub status: String,
    watcher: Option<archipel_ide_project::ProjectWatcher>,
    /// Lets background services (file watcher) wake the UI.
    pub waker: Waker,
    /// Paths queued for the next native file dialog (control socket and tests inject
    /// the user's choice here instead of opening an OS dialog).
    pub injected_dialog_paths: std::collections::VecDeque<PathBuf>,
}

/// Status bar summary after a Git refresh: "1 change on main", "3 changes on main".
pub fn changes_summary(count: usize, branch: &str) -> String {
    let noun = if count == 1 { "change" } else { "changes" };
    format!("{count} {noun} on {branch}")
}

impl IdeState {
    pub fn open_project(&mut self, path: &Path) -> Result<(), String> {
        let project = Project::open(path).map_err(|e| e.to_string())?;
        self.prepare_project_switch()?;
        self.install_project(project)
    }

    /// Saves and closes the open project, returning to the welcome screen.
    pub fn close_project(&mut self) -> Result<(), String> {
        if self.project.is_none() {
            return Err("no project is open".into());
        }
        self.prepare_project_switch()?;
        self.saving().clear_parked();
        self.documents.clear();
        self.active_document = None;
        self.splits = Default::default();
        self.splits_synced = None;
        self.tabs = tabs::EditorTabs {
            limit: self.tabs.limit,
            ..Default::default()
        };
        self.watcher = None;
        self.lsp.set_project(None);
        self.close_diff();
        self.files.clear();
        self.files_generation += 1;
        self.expanded_dirs.clear();
        self.recent_files.clear();
        self.tool_windows = ToolWindowLayout::default();
        self.project = None;
        // Disposes the project services (Git status and its watcher, toolchain requirements,
        // the running task, debug session and shells).
        self.platform.close_project();
        self.status = "Closed project".into();
        Ok(())
    }

    pub(crate) fn prepare_project_switch(&mut self) -> Result<(), String> {
        if self.project.is_some() {
            if self.execution().is_running() {
                return Err("Stop the running task before opening another project.".into());
            }
            // PLAT-SAVE-050: a document that cannot be saved cancels the close or switch.
            self.save_all_at(SavePoint::ProjectClose).into_result()?;
            self.save_workspace()?;
        }
        Ok(())
    }

    pub(crate) fn install_project(&mut self, project: Project) -> Result<(), String> {
        // Closed-tab text belongs to the previous project (it was saved or the switch refused).
        self.saving().clear_parked();
        // Closes the previous project and opens this one; the services below belong to it.
        self.platform.open_project(project.root());
        self.set_files(project.entries(20_000));
        self.documents.clear();
        self.active_document = None;
        self.splits = Default::default();
        self.splits_synced = None;
        self.tabs = tabs::EditorTabs {
            limit: self.tabs.limit,
            ..Default::default()
        };
        if let Some(git) = self.git() {
            git.open(project.root(), self.repaint.notifier());
        }
        self.lsp.set_project(Some(project.root()));
        self.close_diff();
        self.reload_toolchains(Some(project.root()));
        self.refresh_toolchain_resolver();
        self.navigation = NavigationState::default();
        self.code_actions = CodeActionState::default();
        self.refactor = RefactorState::default();
        self.generate = GenerateState::default();
        self.hierarchy = HierarchyState::default();
        self.expanded_dirs.clear();
        self.recent_files.clear();
        self.tool_windows = ToolWindowLayout::default();
        self.tool_windows.show(ToolWindow::Project);
        self.recent_projects().write().record(project.root());
        self.status = format!("Opened {}", project.root().display());
        self.load_project_settings(project.root());
        self.android_tooling().project_opened(project.root());
        self.project = Some(project);
        self.start_watcher();
        self.detect_build();
        self.load_gradle_model();
        self.load_bazel_targets();
        self.open_project_model();
        self.load_run_configurations();
        self.load_search_scopes();
        self.load_breakpoints();
        self.restore_workspace();
        self.load_ai_project();
        self.auto_setup_toolchains();
        Ok(())
    }
    pub fn open_file(&mut self, path: &Path) -> Result<(), String> {
        if self.open_in_plugin_editor(path) {
            return Ok(());
        }
        if let Some(index) = self.documents.iter().position(|d| d.path == path) {
            self.active_document = Some(index);
            self.diff_active = false;
            self.plugin_host_service().set_active_tab(None);
            // Opening a preview tab for real (double-click, Enter) keeps it.
            if self.tabs.preview.as_deref() == Some(path) {
                self.tabs.preview = None;
            }
            self.touch_recent(path);
            self.reveal_in_tree(path);
            return Ok(());
        }
        if self.project.is_none() {
            return Err("no project is open".into());
        }
        let (mut document, text) = match self.take_parked_document(path) {
            // The text of a tab closed without saving comes back as it was.
            Some(document) => {
                let text = document.text.text();
                (document, text)
            }
            None => {
                let project = self.project.as_ref().ok_or("no project is open")?;
                let text = project.read_text(path).map_err(|e| e.to_string())?;
                (Document::new(path.to_owned(), &text), text)
            }
        };
        let defaults = folds::FoldDefaults::from_settings(&self.settings().read());
        document.fold_defaults = defaults;
        document.collapse_default_regions();
        self.documents.push(document);
        self.active_document = Some(self.documents.len() - 1);
        self.diff_active = false;
        self.plugin_host_service().set_active_tab(None);
        self.touch_recent(path);
        self.reveal_in_tree(path);
        self.status = format!("Opened {}", path.display());
        self.sync_document_lsp(path, &text, 0);
        // The disk may have changed since the tab closed with unsaved text.
        if self.documents.last().is_some_and(|d| d.text.is_dirty()) {
            let last = self.documents.len() - 1;
            self.sync_document(last);
        }
        self.enforce_tab_limit()
    }

    /// Expands the Project tree down to `path` so an opened file is visible
    /// (the reference IDE's "Always Select Opened File").
    pub fn reveal_in_tree(&mut self, path: &Path) {
        for ancestor in path.ancestors().skip(1) {
            if !ancestor.as_os_str().is_empty() {
                self.expanded_dirs.insert(ancestor.to_owned());
            }
        }
    }
    /// Closes a tab. Closing does not save and does not ask (PLAT-SAVE-052): modified text
    /// stays in the unsaved set and the next trigger (deactivation, idle, a save point, Save
    /// All) writes it.
    pub fn close_document(&mut self, index: usize) -> Result<(), String> {
        // Tabs opened since the last frame must be in their group before one closes.
        self.sync_splits();
        self.documents.get(index).ok_or("document does not exist")?;
        self.remember_closed(index);
        let document = self.documents.remove(index);
        self.park_closed_document(document.clone());
        self.active_document = match self.active_document {
            _ if self.documents.is_empty() => None,
            Some(active) if active > index => Some(active - 1),
            Some(active) if active == index => Some(index.min(self.documents.len() - 1)),
            other => other,
        };
        if self.tabs.preview.as_ref() == Some(&document.path) {
            self.tabs.preview = None;
        }
        self.close_tab_in_splits(&document.path);
        self.close_completion();
        if let Some(project) = &self.project {
            let _ = self
                .lsp
                .close_document(&project.root().join(&document.path));
        }
        self.status = format!("Closed {}", document.path.display());
        Ok(())
    }

    pub fn activate_document(&mut self, index: usize) -> Result<(), String> {
        if index >= self.documents.len() {
            return Err("document does not exist".into());
        }
        self.active_document = Some(index);
        self.diff_active = false;
        self.plugin_host_service().set_active_tab(None);
        let path = self.documents[index].path.clone();
        self.touch_recent(&path);
        self.close_completion();
        Ok(())
    }

    /// Moves `path` to the front of the recent-files list.
    pub fn touch_recent(&mut self, path: &Path) {
        self.recent_files.retain(|recent| recent != path);
        self.recent_files.insert(0, path.to_owned());
        self.recent_files.truncate(50);
    }

    /// Expands or collapses a directory in the Project tool window.
    /// Returns whether the directory is expanded afterwards.
    pub fn toggle_directory(&mut self, path: &Path) -> Result<bool, String> {
        if !self
            .files
            .iter()
            .any(|e| e.is_dir && e.relative_path == path)
        {
            return Err(format!("{} is not a project directory", path.display()));
        }
        if self.expanded_dirs.remove(path) {
            Ok(false)
        } else {
            self.load_children(path)?;
            self.expanded_dirs.insert(path.to_owned());
            Ok(true)
        }
    }

    /// Lists a directory the initial walk did not descend into (symbolic links
    /// such as `bazel-bin`, and directories below them) when it is expanded.
    fn load_children(&mut self, path: &Path) -> Result<(), String> {
        let loaded = self
            .files
            .iter()
            .any(|entry| entry.relative_path.parent() == Some(path));
        let linked = path.ancestors().any(|ancestor| {
            self.files
                .iter()
                .any(|entry| (entry.is_symlink || entry.lazy) && entry.relative_path == ancestor)
        });
        if loaded || !linked {
            return Ok(());
        }
        let project = self.project.as_ref().ok_or("no project is open")?;
        let children = project.children(path).map_err(|error| error.to_string())?;
        let Some(position) = self.files.iter().position(|e| e.relative_path == path) else {
            return Ok(());
        };
        self.files.splice(position + 1..position + 1, children);
        Ok(())
    }

    /// Entries visible in the Project tree: every ancestor directory is expanded.
    pub fn visible_files(&self) -> Vec<&FileEntry> {
        self.files
            .iter()
            .filter(|entry| {
                entry
                    .relative_path
                    .ancestors()
                    .skip(1)
                    .filter(|ancestor| !ancestor.as_os_str().is_empty())
                    .all(|ancestor| self.expanded_dirs.contains(ancestor))
            })
            .collect()
    }

    pub fn refresh_active_analysis(&mut self) {
        if let Some(document) = self
            .active_document
            .and_then(|index| self.documents.get_mut(index))
        {
            let source = document.text.text();
            document.refresh_analysis(&source);
        }
    }

    pub fn open_terminal(&mut self) -> Result<(), String> {
        self.open_terminal_in(None)
    }

    /// Opens the Terminal; a new shell starts in `dir` (default: the project
    /// root) and a running shell is sent a `cd` there.
    pub fn open_terminal_in(&mut self, dir: Option<&Path>) -> Result<(), String> {
        let terminal = self.terminal();
        let running = terminal
            .session()
            .as_ref()
            .is_some_and(|terminal| !terminal.is_exited());
        if let Some(dir) = dir
            && running
        {
            let quoted = dir.display().to_string().replace('\'', "'\\''");
            if let Some(session) = terminal.session_mut().as_mut() {
                session
                    .send(&format!("cd '{quoted}'\n"))
                    .map_err(|error| error.to_string())?;
            }
        } else if !running {
            let session = self.spawn_terminal_session(dir)?;
            terminal.state_mut().set_session(session);
        }
        self.tool_windows.show(ToolWindow::Terminal);
        self.status = "Terminal ready".into();
        Ok(())
    }

    /// Text occurrences for Search Everywhere; runs next to (and never replaces)
    /// the Find in Files results.
    pub fn request_text_search(&mut self, query: &str) {
        let query = query.trim();
        let nav = self.search_everywhere();
        if nav.read().text_query == query {
            return;
        }
        {
            let mut nav = nav.write();
            nav.text.lock().cancel();
            nav.text_results.clear();
            nav.text_query = query.to_owned();
        }
        let Some(project) = self.project.as_ref() else {
            return;
        };
        if query.chars().count() < 2 {
            return;
        }
        let files = self
            .files
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.relative_path.clone())
            .collect();
        // An unstartable query (empty) simply lists no text hits.
        let _ = nav.read().text.lock().start(
            project.root().to_owned(),
            files,
            query.to_owned(),
            &SearchOptions::default(),
        );
    }

    /// Directory of the Gradle module that contains the active file, relative
    /// to the project root (`""` for the root module).
    pub fn active_module_dir(&self) -> Option<PathBuf> {
        let root = self.project.as_ref()?.root();
        let active = &self.documents.get(self.active_document?)?.path;
        let active = active.strip_prefix(root).unwrap_or(active);
        self.gradle_model
            .as_ref()?
            .projects
            .iter()
            .filter_map(|module| module.dir.strip_prefix(root).ok())
            .filter(|dir| active.starts_with(dir))
            .max_by_key(|dir| dir.components().count())
            .map(Path::to_owned)
    }

    fn in_search_scope(
        &self,
        path: &Path,
        custom: Option<&archipel_ide_search::FileMaskFilter>,
    ) -> bool {
        let under_mark = |tests: bool| {
            self.directory_marks().iter().any(|(dir, mark)| {
                path.starts_with(dir)
                    && matches!(
                        mark,
                        project_view::DirMark::Tests | project_view::DirMark::TestResources
                    ) == tests
                    && !matches!(mark, project_view::DirMark::Excluded)
            })
        };
        let scope_kind = self.find().map(|find| find.read().scope_kind);
        match scope_kind.unwrap_or_default() {
            SearchScopeKind::Project => true,
            SearchScopeKind::Directory => self
                .project_view
                .search_scope
                .as_ref()
                .is_none_or(|scope| path.starts_with(scope)),
            SearchScopeKind::Module => self
                .active_module_dir()
                .is_none_or(|dir| path.starts_with(dir)),
            SearchScopeKind::OpenFiles => self.documents.iter().any(|document| {
                let root = self.project.as_ref().map(|project| project.root());
                root.and_then(|root| document.path.strip_prefix(root).ok())
                    .unwrap_or(&document.path)
                    == path
            }),
            SearchScopeKind::TestFiles => under_mark(true),
            SearchScopeKind::ProductionFiles => !under_mark(true),
            SearchScopeKind::Custom => custom.is_some_and(|filter| filter.accepts(path)),
        }
    }

    pub fn poll_text_search(&mut self) {
        let nav = self.search_everywhere();
        let mut nav = nav.write();
        let events = nav.text.lock().poll();
        for event in events {
            if let SearchEvent::Match(found) = event {
                nav.text_results.push(found);
            }
        }
    }

    /// Starts a shell in `dir` (default: the project root) as the Terminal
    /// settings describe it.
    pub(crate) fn spawn_terminal_session(
        &self,
        dir: Option<&Path>,
    ) -> Result<TerminalSession, String> {
        let cwd = dir
            .or_else(|| self.project.as_ref().map(Project::root))
            .unwrap_or_else(|| Path::new("."));
        let shell = self
            .settings()
            .read()
            .text(archipel_ide_settings::ids::TERMINAL_SHELL)
            .unwrap_or_default();
        if shell.trim().is_empty() {
            TerminalSession::spawn_shell(cwd).map_err(|e| e.to_string())
        } else {
            TerminalSession::spawn(shell.trim(), &[], cwd)
                .map_err(|e| format!("{e}. Check the shell path in Settings | Tools | Terminal."))
        }
    }

    pub fn start_search(&mut self, query: String) -> Result<u64, String> {
        self.start_search_with(query, SearchOptions::default())
    }

    /// Find in Files with match-case, whole-word, regex and file-mask options.
    /// Project files in the current Find in Files scope.
    pub(crate) fn search_files(&self) -> Vec<PathBuf> {
        let custom = self.custom_scope_filter();
        self.files
            .iter()
            .filter(|entry| !entry.is_dir)
            .filter(|entry| self.in_search_scope(&entry.relative_path, custom.as_ref()))
            .map(|entry| entry.relative_path.clone())
            .collect()
    }

    pub fn start_search_with(
        &mut self,
        query: String,
        options: SearchOptions,
    ) -> Result<u64, String> {
        // Validate before clearing the previous results.
        archipel_ide_search::Matcher::new(&query, &options).map_err(|error| error.to_string())?;
        let project = self.project.as_ref().ok_or("no project is open")?;
        let find = self.find().ok_or("no project is open")?;
        let files = self.search_files();
        let background = find.read().background;
        if !background {
            self.tool_windows.show(ToolWindow::Find);
        }
        let job = find
            .start(project.root().to_owned(), files, query, options)
            .map_err(|error| error.to_string())?;
        self.status = format!("Searching project (job {job})");
        Ok(job)
    }

    /// Collects the results the search thread found; the status bar follows the
    /// [`services::FIND_TOPIC`] event the service publishes when the search finishes.
    pub fn poll_search(&mut self) {
        if let Some(find) = self.find() {
            find.poll();
        }
        self.dispatch_reactions();
    }

    pub fn refresh_git(&mut self) -> Result<(), String> {
        let root = self.project.as_ref().ok_or("no project is open")?.root();
        let status = RepositoryStatus::load(root).map_err(|error| error.to_string())?;
        self.status = changes_summary(status.changes.len(), &status.branch);
        // Announces the change; `dispatch_reactions` refreshes what depends on it.
        self.git().ok_or("no project is open")?.update(status);
        self.dispatch_reactions();
        Ok(())
    }

    pub fn stage_path(&mut self, path: &Path) -> Result<(), String> {
        self.git_status()
            .ok_or("project is not a Git repository")?
            .stage(path)
            .map_err(|error| error.to_string())?;
        self.refresh_git()
    }

    pub fn unstage_path(&mut self, path: &Path) -> Result<(), String> {
        self.git_status()
            .ok_or("project is not a Git repository")?
            .unstage(path)
            .map_err(|error| error.to_string())?;
        self.refresh_git()
    }

    /// Pumps the language servers and re-opens documents on restarted servers.
    pub fn poll_lsp(&mut self) {
        let restarted = self.lsp.poll();
        self.poll_completion();
        self.sync_external_diagnostics();
        self.refresh_problems();
        self.poll_navigation();
        self.poll_code_actions();
        self.poll_refactor();
        self.poll_generate();
        self.poll_hierarchy();
        self.poll_docs();
        self.poll_signature();
        self.poll_code_vision();
        let Some(root) = self.project.as_ref().map(|p| p.root().to_owned()) else {
            return;
        };
        for absolute in restarted {
            let Some(document) = self
                .documents
                .iter()
                .find(|document| root.join(&document.path) == absolute)
            else {
                continue;
            };
            let version = document.text.revision().min(i32::MAX as u64) as i32;
            let _ = self
                .lsp
                .sync_document(&absolute, &document.text.text(), version);
        }
    }

    /// Opens or updates a document on its language server, starting the
    /// server on first use. A missing server raises a notice, not an error.
    pub fn sync_document_lsp(&mut self, relative_path: &Path, text: &str, version: i32) {
        let Some(project) = &self.project else {
            return;
        };
        let absolute = project.root().join(relative_path);
        match self.lsp.sync_document(&absolute, text, version) {
            Ok(()) | Err(archipel_ide_lsp::LspError::ServerNotFound { .. }) => {}
            Err(error) => self.status = error.to_string(),
        }
    }

    pub fn sync_active_lsp(&mut self) {
        let Some(index) = self.active_document else {
            return;
        };
        let Some(document) = self.documents.get(index) else {
            return;
        };
        let path = document.path.clone();
        let text = document.text.text();
        let version = document.text.revision().min(i32::MAX as u64) as i32;
        self.sync_document_lsp(&path, &text, version);
    }

    pub fn active_caret(&self) -> Option<usize> {
        self.active_document
            .and_then(|index| self.documents.get(index))
            .map(|document| document.selections.primary().head)
    }

    /// Runs an edit on the active document and refreshes analysis/LSP if it changed.
    pub fn edit_active<T>(
        &mut self,
        edit: impl FnOnce(&mut Document) -> Result<T, String>,
    ) -> Result<T, String> {
        let index = self.active_document.ok_or("no active document")?;
        let document = self
            .documents
            .get_mut(index)
            .ok_or("active document does not exist")?;
        let revision = document.text.revision();
        let result = edit(document)?;
        if document.text.revision() != revision {
            let source = document.text.text();
            document.refresh_analysis(&source);
            self.sync_active_lsp();
            self.close_completion();
            self.record_change_place();
        }
        Ok(result)
    }

    pub fn insert_active_text(&mut self, position: usize, text: &str) -> Result<(), String> {
        let index = self.active_document.ok_or("no active document")?;
        let document = self
            .documents
            .get_mut(index)
            .ok_or("active document does not exist")?;
        document
            .text
            .edit(position, position, text)
            .map_err(|e| e.to_string())?;
        let source = document.text.text();
        document.refresh_analysis(&source);
        document
            .selections
            .set_single(Selection::caret(position + text.chars().count()));
        self.sync_active_lsp();
        self.close_completion();
        Ok(())
    }

    pub fn active_lsp_diagnostics(&self) -> Vec<LspDiagnostic> {
        let Some(index) = self.active_document else {
            return Vec::new();
        };
        let Some(document) = self.documents.get(index) else {
            return Vec::new();
        };
        let Some(project) = &self.project else {
            return Vec::new();
        };
        self.lsp
            .diagnostics_for(&project.root().join(&document.path))
    }
}

pub(crate) fn spec(
    id: &str,
    label: &str,
    category: &str,
    shortcut: Option<&str>,
    params: &str,
) -> CommandSpec {
    CommandSpec {
        id: id.into(),
        label: label.into(),
        category: category.into(),
        shortcut: shortcut.map(Into::into),
        params: params.into(),
    }
}
pub(crate) fn string_param(params: &Value, name: &str) -> Result<String, CommandError> {
    params
        .get(name)
        .and_then(Value::as_str)
        .map(Into::into)
        .ok_or_else(|| CommandError::InvalidParams(format!("expected string field `{name}`")))
}

pub(crate) fn string_array_param(params: &Value, name: &str) -> Result<Vec<String>, CommandError> {
    params
        .get(name)
        .map(|value| {
            value
                .as_array()
                .ok_or_else(|| {
                    CommandError::InvalidParams(format!("expected array field `{name}`"))
                })?
                .iter()
                .map(|item| {
                    item.as_str().map(Into::into).ok_or_else(|| {
                        CommandError::InvalidParams(format!("expected strings in `{name}`"))
                    })
                })
                .collect()
        })
        .transpose()
        .map(|value| value.unwrap_or_default())
}

fn tool_window_param(params: &Value) -> Result<ToolWindow, CommandError> {
    let id = string_param(params, "id")?;
    ToolWindow::parse(&id)
        .ok_or_else(|| CommandError::InvalidParams(format!("unknown tool window `{id}`")))
}

pub(crate) fn usize_param(params: &Value, name: &str) -> Result<usize, CommandError> {
    params
        .get(name)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| {
            CommandError::InvalidParams(format!("expected non-negative integer field `{name}`"))
        })
}

pub fn built_in_commands() -> CommandRegistry<IdeState> {
    let mut registry = CommandRegistry::default();
    agents::register(&mut registry);
    android_commands::register(&mut registry);
    android_devices::register(&mut registry);
    android_deploy::register(&mut registry);
    android_variants::register(&mut registry);
    android_resources::register(&mut registry);
    android_tools::register(&mut registry);
    android_manifest::register(&mut registry);
    android_logcat::register(&mut registry);
    android_sdk_repo::register(&mut registry);
    bazel_commands::register(&mut registry);
    build_commands::register(&mut registry);
    debug_commands::register(&mut registry);
    editor_commands::register(&mut registry);
    editor_input::register(&mut registry);
    markdown_commands::register(&mut registry);
    disk_sync::register(&mut registry);
    tabs::register(&mut registry);
    gradle_commands::register(&mut registry);
    project_model::register(&mut registry);
    gutter_commands::register(&mut registry);
    run_config_commands::register(&mut registry);
    search_scopes::register(&mut registry);
    folds::register(&mut registry);
    project_commands::register(&mut registry);
    spring_commands::register(&mut registry);
    project_view::register(&mut registry);
    new_class::register(&mut registry);
    update_commands::register(&mut registry);
    diagnostic_commands::register(&mut registry);
    terminal_commands::register(&mut registry);
    terminal_tabs::register(&mut registry);
    settings_commands::register(&mut registry);
    quick_nav::register(&mut registry);
    keymap_commands::register(&mut registry);
    live_templates::register(&mut registry);
    first_run::register(&mut registry);
    lsp_commands::register(&mut registry);
    vcs::register(&mut registry);
    plugin_host::register(&mut registry);
    problems::register(&mut registry);
    automation_commands::register(&mut registry);
    toolchain_commands::register(&mut registry);
    line_changes::register(&mut registry);
    git_log::register(&mut registry);
    branch_commands::register(&mut registry);
    remote_commands::register(&mut registry);
    welcome_commands::register(&mut registry);
    annotations::register(&mut registry);
    code_vision::register(&mut registry);
    build_window::register(&mut registry);
    worktree_commands::register(&mut registry);
    search_replace::register(&mut registry);
    navigation::register(&mut registry);
    code_actions::register(&mut registry);
    refactor::register(&mut registry);
    generate::register(&mut registry);
    hierarchy::register(&mut registry);
    test_nav::register(&mut registry);
    ai_commands::register(&mut registry);
    analysis_modules::register_commands(&mut registry);
    registry
        .register(Command::new(
            spec(
                "project.open",
                "Open Project",
                "Project",
                Some("⌘O"),
                r#"{"path":"string"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                state
                    .open_project(Path::new(&string_param(&params, "path")?))
                    .map_err(CommandError::Failed)?;
                let root = state
                    .project
                    .as_ref()
                    .map(|project| project.root().to_owned())
                    .ok_or_else(|| CommandError::Failed("no project is open".into()))?;
                Ok(json!({"root": root}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("project.close", "Close Project", "Project", None, "{}"),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                state.close_project().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.insertText",
                "Insert Text",
                "Editor",
                None,
                r#"{"position":0,"text":"string"}"#,
            ),
            |state: &IdeState| state.active_document.is_some(),
            |state: &mut IdeState, params| {
                let position = usize_param(&params, "position")?;
                let text = string_param(&params, "text")?;
                state
                    .insert_active_text(position, &text)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("git.refresh", "Refresh Git Status", "Git", None, "{}"),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, _| {
                // PLAT-SAVE-041: a change-list refresh saves the documents first.
                state.save_before(SavePoint::Vcs);
                state.refresh_git().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.stage",
                "Stage File",
                "Git",
                None,
                r#"{"path":"string","hunks":"[usize]?"}"#,
            ),
            |state: &IdeState| state.has_git(),
            |state: &mut IdeState, params| {
                let path = string_param(&params, "path")?;
                // With `hunks`, only those chunks of the open diff are staged.
                match params.get("hunks").and_then(Value::as_array) {
                    Some(hunks) => {
                        let hunks: Vec<usize> = hunks
                            .iter()
                            .filter_map(|hunk| hunk.as_u64().map(|hunk| hunk as usize))
                            .collect();
                        state
                            .stage_hunks(Path::new(&path), &hunks)
                            .map_err(CommandError::Failed)?;
                    }
                    None => state
                        .stage_path(Path::new(&path))
                        .map_err(CommandError::Failed)?,
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "git.unstage",
                "Unstage File",
                "Git",
                None,
                r#"{"path":"string"}"#,
            ),
            |state: &IdeState| state.has_git(),
            |state: &mut IdeState, params| {
                state
                    .unstage_path(Path::new(&string_param(&params, "path")?))
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "search.project",
                "Find in Files",
                "Search",
                Some("⇧⌘F"),
                r#"{"query":"string","matchCase":"bool?","wholeWords":"bool?","regex":"bool?","fileMask":"string?","scope":"string?","scopeKind":"string?","scopeName":"string?","background":"bool?"}"#,
            ),
            |state: &IdeState| state.project.is_some(),
            |state: &mut IdeState, params| {
                let query = string_param(&params, "query")?;
                let flag = |name: &str| params.get(name).and_then(Value::as_bool).unwrap_or(false);
                let options = SearchOptions {
                    match_case: flag("matchCase"),
                    whole_words: flag("wholeWords"),
                    regex: flag("regex"),
                    file_mask: params
                        .get("fileMask")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                };
                // `scope` limits the search to a directory ("Find in Files" on a folder).
                state.project_view.search_scope = params
                    .get("scope")
                    .and_then(Value::as_str)
                    .filter(|scope| !scope.is_empty())
                    .map(PathBuf::from);
                let scope_kind = params
                    .get("scopeKind")
                    .and_then(Value::as_str)
                    .and_then(SearchScopeKind::parse)
                    .unwrap_or(if state.project_view.search_scope.is_some() {
                        SearchScopeKind::Directory
                    } else {
                        SearchScopeKind::Project
                    });
                let scope_name = params
                    .get("scopeName")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                if let Some(find) = state.find() {
                    let mut find = find.write();
                    find.scope_kind = scope_kind;
                    find.custom_scope = scope_name;
                    find.background = flag("background");
                }
                let job = state.start_search_with(query, options);
                if let Some(find) = state.find() {
                    find.write().background = false;
                }
                let job = job.map_err(CommandError::InvalidParams)?;
                Ok(json!({"job": job}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("search.cancel", "Cancel Search", "Search", None, "{}"),
            |state: &IdeState| state.search_running(),
            |state: &mut IdeState, _| {
                if let Some(find) = state.find() {
                    find.cancel();
                }
                state.status = "Search cancelled".into();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "terminal.open",
                "Open Terminal",
                "Terminal",
                Some("⌥F12"),
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.open_terminal().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("terminal.toggle", "Toggle Terminal", "Terminal", None, "{}"),
            |_| true,
            |state: &mut IdeState, _| {
                if !state.terminal().has_session() {
                    state.open_terminal().map_err(CommandError::Failed)?;
                } else {
                    state.tool_windows.toggle(ToolWindow::Terminal);
                }
                Ok(json!({"visible": state.tool_windows.is_visible(ToolWindow::Terminal)}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "terminal.sendText",
                "Send Text to Terminal",
                "Terminal",
                None,
                r#"{"text":"string"}"#,
            ),
            |state: &IdeState| state.terminal().has_session(),
            |state: &mut IdeState, params| {
                let text = string_param(&params, "text")?;
                state
                    .terminal()
                    .session_mut()
                    .as_mut()
                    .ok_or_else(|| CommandError::Disabled("terminal.sendText".into()))?
                    .send(&text)
                    .map_err(|e| CommandError::Failed(e.to_string()))?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    splits::register(&mut registry);
    registry
        .register(Command::new(
            spec(
                "editor.openFile",
                "Open File",
                "Editor",
                None,
                r#"{"path":"string","line":"number?","column":"number?","length":"number?"}"#,
            ),
            |s: &IdeState| s.project.is_some(),
            |state: &mut IdeState, params| {
                state
                    .open_file(Path::new(&string_param(&params, "path")?))
                    .map_err(CommandError::Failed)?;
                // Navigation targets (Find in Files, diagnostics) land on the
                // match and select it.
                if params.get("line").is_some() {
                    let line = usize_param(&params, "line")?;
                    let column = params.get("column").and_then(Value::as_u64).unwrap_or(1);
                    let length = params.get("length").and_then(Value::as_u64).unwrap_or(0);
                    editor_commands::select_in_active(
                        state,
                        line,
                        column as usize,
                        length as usize,
                    )
                    .map_err(CommandError::Failed)?;
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.save", "Save", "Editor", None, "{}"),
            |s: &IdeState| s.active_document.is_some(),
            |state: &mut IdeState, _| {
                state
                    .save(
                        state
                            .active_document
                            .ok_or_else(|| CommandError::Disabled("editor.save".into()))?,
                    )
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.saveAll", "Save All", "Editor", Some("⌘S"), "{}"),
            |s: &IdeState| !s.unsaved_documents().is_empty(),
            |state: &mut IdeState, _| {
                state.save_all().map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.undo", "Undo", "Editor", Some("⌘Z"), "{}"),
            |s: &IdeState| {
                s.active_document
                    .and_then(|i| s.documents.get(i))
                    .is_some_and(|d| d.text.can_undo())
            },
            |state: &mut IdeState, _| {
                if state.undo_active() {
                    state.refresh_active_analysis();
                    state.sync_active_lsp();
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec("editor.redo", "Redo", "Editor", Some("⇧⌘Z"), "{}"),
            |s: &IdeState| {
                s.active_document
                    .and_then(|i| s.documents.get(i))
                    .is_some_and(|d| d.text.can_redo())
            },
            |state: &mut IdeState, _| {
                if state.redo_active() {
                    state.refresh_active_analysis();
                    state.sync_active_lsp();
                }
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolwindow.toggle",
                "Toggle Tool Window",
                "Window",
                None,
                r#"{"id":"project|commit|find|build|problems|terminal|deviceManager|logcat|buildVariants|resourceManager|runningDevices"}"#,
            ),
            |_| true,
            |state: &mut IdeState, params| {
                let window = tool_window_param(&params)?;
                if window == ToolWindow::Terminal && !state.terminal().has_session() {
                    state.open_terminal().map_err(CommandError::Failed)?;
                    return Ok(json!({"visible": true}));
                }
                if window == ToolWindow::Commit && !state.tool_windows.is_visible(window) {
                    let _ = state.refresh_git();
                }
                if window == ToolWindow::DeviceManager && !state.tool_windows.is_visible(window) {
                    state.device_manager_opened();
                }
                if window == ToolWindow::Logcat && !state.tool_windows.is_visible(window) {
                    state.logcat_opened();
                }
                if window == ToolWindow::ResourceManager && !state.tool_windows.is_visible(window) {
                    state.resource_manager_opened();
                }
                if window == ToolWindow::RunningDevices && !state.tool_windows.is_visible(window) {
                    state.running_devices_opened();
                }
                Ok(json!({"visible": state.tool_windows.toggle(window)}))
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "toolwindow.hideAll",
                "Hide All Tool Windows",
                "Window",
                Some("⇧⌘F12"),
                "{}",
            ),
            |_| true,
            |state: &mut IdeState, _| {
                state.tool_windows.hide_all();
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    for window in ToolWindow::ALL {
        let id = format!("toolwindow.activate.{}", window.id());
        registry
            .register(Command::new(
                spec(
                    &id,
                    &format!("{} Tool Window", window.title()),
                    "Window",
                    Some(window.shortcut()).filter(|shortcut| !shortcut.is_empty()),
                    "{}",
                ),
                |_| true,
                move |state: &mut IdeState, _| {
                    if window == ToolWindow::DeviceManager {
                        state.device_manager_opened();
                    }
                    if window == ToolWindow::Logcat {
                        state.logcat_opened();
                    }
                    if window == ToolWindow::ResourceManager {
                        state.resource_manager_opened();
                    }
                    if window == ToolWindow::RunningDevices {
                        state.running_devices_opened();
                    }
                    if window == ToolWindow::Terminal {
                        state.open_terminal().map_err(CommandError::Failed)?;
                    }
                    if window == ToolWindow::Commit {
                        let _ = state.refresh_git();
                    }
                    state.tool_windows.show(window);
                    Ok(Value::Null)
                },
            ))
            .expect("unique built-in id");
    }
    registry
        .register(Command::new(
            spec(
                "editor.close",
                "Close Tab",
                "Editor",
                Some("⌘W"),
                r#"{"index":"number?"}"#,
            ),
            |s: &IdeState| s.active_document.is_some(),
            |state: &mut IdeState, params| {
                let index = match params.get("index") {
                    Some(_) => usize_param(&params, "index")?,
                    None => state
                        .active_document
                        .ok_or_else(|| CommandError::Disabled("editor.close".into()))?,
                };
                state.close_document(index).map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "editor.activate",
                "Select Tab",
                "Editor",
                None,
                r#"{"index":"number"}"#,
            ),
            |s: &IdeState| !s.documents.is_empty(),
            |state: &mut IdeState, params| {
                state
                    .activate_document(usize_param(&params, "index")?)
                    .map_err(CommandError::Failed)?;
                Ok(Value::Null)
            },
        ))
        .expect("unique built-in id");
    registry
        .register(Command::new(
            spec(
                "project.toggleDirectory",
                "Expand/Collapse Directory",
                "Project",
                None,
                r#"{"path":"string"}"#,
            ),
            |s: &IdeState| s.project.is_some(),
            |state: &mut IdeState, params| {
                let expanded = state
                    .toggle_directory(Path::new(&string_param(&params, "path")?))
                    .map_err(CommandError::Failed)?;
                Ok(json!({"expanded": expanded}))
            },
        ))
        .expect("unique built-in id");
    registry
}

#[cfg(test)]
mod tests {
    #[test]
    fn change_summary_uses_singular_for_one_change() {
        assert_eq!(super::changes_summary(1, "master"), "1 change on master");
        assert_eq!(super::changes_summary(0, "main"), "0 changes on main");
        assert_eq!(super::changes_summary(2, "main"), "2 changes on main");
    }

    #[cfg(unix)]
    #[test]
    fn linked_directories_expand_and_their_files_open() {
        use super::*;
        let base =
            std::env::temp_dir().join(format!("archipel-ide-core-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("out/pkg")).unwrap();
        std::fs::write(base.join("out/pkg/App.class.txt"), "bytes").unwrap();
        std::fs::create_dir_all(base.join("ws")).unwrap();
        std::os::unix::fs::symlink(base.join("out"), base.join("ws/bazel-bin")).unwrap();
        let registry = built_in_commands();
        let mut state = IdeState::default();
        state.open_project(&base.join("ws")).unwrap();
        let visible = |state: &IdeState| -> Vec<String> {
            state
                .visible_files()
                .iter()
                .map(|e| e.relative_path.display().to_string())
                .collect()
        };
        assert_eq!(visible(&state), ["bazel-bin"]);
        for dir in ["bazel-bin", "bazel-bin/pkg"] {
            registry
                .execute("project.toggleDirectory", &mut state, json!({"path": dir}))
                .unwrap();
        }
        assert_eq!(
            visible(&state),
            ["bazel-bin", "bazel-bin/pkg", "bazel-bin/pkg/App.class.txt"]
        );
        // Collapsing and expanding again does not duplicate children.
        for _ in 0..2 {
            registry
                .execute(
                    "project.toggleDirectory",
                    &mut state,
                    json!({"path": "bazel-bin"}),
                )
                .unwrap();
        }
        assert_eq!(visible(&state).len(), 3);
        registry
            .execute(
                "editor.openFile",
                &mut state,
                json!({"path": "bazel-bin/pkg/App.class.txt"}),
            )
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "bytes");
        let _ = std::fs::remove_dir_all(base);
    }

    use super::*;
    use std::{
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn document_analysis_follows_edits_and_undo_incrementally() {
        let mut document = Document::new("App.java".into(), "class App { int x = 1; }\n");
        document.text.edit(20, 21, "\"s\"").unwrap();
        let source = document.text.text();
        document.refresh_analysis(&source);
        let full = archipel_ide_language::analyze(&document.path, &source);
        assert_eq!(document.analysis.highlights, full.highlights);
        assert!(document.text.undo());
        let source = document.text.text();
        document.refresh_analysis(&source);
        assert_eq!(
            document.analysis.highlights,
            archipel_ide_language::analyze(&document.path, &source).highlights
        );
    }

    #[test]
    fn project_open_command_is_shared_engine_behavior() {
        let root = std::env::temp_dir().join(format!("archipel-ide-core-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        assert!(state.project.is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn switching_projects_saves_documents_and_restores_the_previous_workspace() {
        let root = std::env::temp_dir().join(format!("archipel-ide-switch-{}", std::process::id()));
        let first = root.join("first");
        let second = root.join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("notes.txt"), "before").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": first}))
            .unwrap();
        state.open_file(Path::new("notes.txt")).unwrap();
        state.documents[0].text.edit(0, 6, "after").unwrap();
        state.find().unwrap().write().query = "old query".into();
        commands
            .execute("project.open", &mut state, json!({"path": second}))
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(first.join("notes.txt")).unwrap(),
            "after"
        );
        assert!(state.documents.is_empty());
        assert!(state.search_query().is_empty());
        commands
            .execute("project.open", &mut state, json!({"path": first}))
            .unwrap();
        assert_eq!(state.documents[0].text.text(), "after");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_project_open_preserves_the_current_editor() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-switch-error-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("notes.txt"), "before").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        state.open_file(Path::new("notes.txt")).unwrap();
        state.documents[0].text.edit(0, 6, "unsaved").unwrap();
        assert!(
            commands
                .execute(
                    "project.open",
                    &mut state,
                    json!({"path": root.join("missing")})
                )
                .is_err()
        );
        assert_eq!(state.documents[0].text.text(), "unsaved");
        assert!(state.documents[0].text.is_dirty());
        assert_eq!(
            std::fs::read_to_string(root.join("notes.txt")).unwrap(),
            "before"
        );
        // A failed save also prevents discarding the old project.
        std::fs::remove_file(root.join("notes.txt")).unwrap();
        std::fs::create_dir(root.join("notes.txt")).unwrap();
        let next = root.join("next");
        std::fs::create_dir(&next).unwrap();
        assert!(
            commands
                .execute("project.open", &mut state, json!({"path": next}))
                .is_err()
        );
        assert_eq!(
            state.project.as_ref().unwrap().root(),
            root.canonicalize().unwrap()
        );
        assert!(state.documents[0].text.is_dirty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_search_command_streams_results_into_state() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("sample.txt"), "alpha needle omega\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("search.project", &mut state, json!({"query": "needle"}))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while state.search_running() && Instant::now() < deadline {
            state.poll_search();
            thread::sleep(Duration::from_millis(5));
        }
        state.poll_search();
        assert_eq!(state.search_result_count(), 1);
        assert_eq!(state.search_results()[0].path, Path::new("sample.txt"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_search_honours_options_and_rejects_bad_regex() {
        use super::*;
        let root = std::env::temp_dir().join(format!(
            "archipel-ide-core-search-opts-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("App.java"), "Greet greet\ngreeting\n").unwrap();
        std::fs::write(root.join("notes.txt"), "greet\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        let run = |state: &mut IdeState, params: Value| {
            commands.execute("search.project", state, params).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            while state.search_running() && Instant::now() < deadline {
                state.poll_search();
                thread::sleep(Duration::from_millis(5));
            }
            state.poll_search();
            state
                .search_results()
                .iter()
                .map(|result| (result.path.display().to_string(), result.line))
                .collect::<Vec<_>>()
        };
        let hits = run(
            &mut state,
            json!({"query": "greet", "matchCase": true, "wholeWords": true, "fileMask": "*.java"}),
        );
        assert_eq!(hits, [("App.java".to_owned(), 1)]);
        assert_eq!(state.search_results()[0].highlights, vec![6..11]);
        assert!(state.search_options().match_case && state.search_options().whole_words);
        let hits = run(&mut state, json!({"query": "^greet", "regex": true}));
        assert_eq!(hits.len(), 3, "{hits:?}");
        let previous = state.search_result_count();
        let error = commands
            .execute(
                "search.project",
                &mut state,
                json!({"query": "(", "regex": true}),
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("invalid regular expression"),
            "{error}"
        );
        assert_eq!(
            state.search_result_count(),
            previous,
            "bad input keeps old results"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn insert_text_command_is_transactional_and_updates_the_caret() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-insert-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("sample.rs"), "fn main() {}\n").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        commands
            .execute("editor.openFile", &mut state, json!({"path": "sample.rs"}))
            .unwrap();
        commands
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 3, "text": "async "}),
            )
            .unwrap();

        assert_eq!(state.documents[0].text.text(), "fn async main() {}\n");
        assert_eq!(state.documents[0].selections.primary().head, 9);
        assert!(state.documents[0].text.can_undo());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn project_tree_starts_collapsed_and_expands_by_command() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src/nested")).unwrap();
        std::fs::write(root.join("src/nested/deep.rs"), "").unwrap();
        std::fs::write(root.join("README.md"), "").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        let visible = |state: &IdeState| {
            state
                .visible_files()
                .into_iter()
                .map(|entry| entry.relative_path.to_string_lossy().replace('\\', "/"))
                .collect::<Vec<_>>()
        };
        assert_eq!(visible(&state), ["src", "README.md"]);
        commands
            .execute(
                "project.toggleDirectory",
                &mut state,
                json!({"path": "src"}),
            )
            .unwrap();
        assert_eq!(visible(&state), ["src", "src/nested", "README.md"]);
        assert!(
            commands
                .execute(
                    "project.toggleDirectory",
                    &mut state,
                    json!({"path": "README.md"})
                )
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn closing_a_tab_keeps_a_sensible_active_tab_and_does_not_save() {
        let root =
            std::env::temp_dir().join(format!("archipel-ide-core-close-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "a").unwrap();
        std::fs::write(root.join("b.txt"), "b").unwrap();
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("project.open", &mut state, json!({"path": root}))
            .unwrap();
        for path in ["a.txt", "b.txt"] {
            commands
                .execute("editor.openFile", &mut state, json!({"path": path}))
                .unwrap();
        }
        commands
            .execute(
                "editor.insertText",
                &mut state,
                json!({"position": 1, "text": "!"}),
            )
            .unwrap();
        commands
            .execute("editor.close", &mut state, Value::Null)
            .unwrap();
        // PLAT-SAVE-052: closing a tab does not save; the text waits for the next trigger.
        assert_eq!(std::fs::read_to_string(root.join("b.txt")).unwrap(), "b");
        assert_eq!(state.unsaved_documents(), vec![PathBuf::from("b.txt")]);
        state.save_all().unwrap();
        assert_eq!(std::fs::read_to_string(root.join("b.txt")).unwrap(), "b!");
        assert_eq!(state.documents.len(), 1);
        assert_eq!(state.active_document, Some(0));
        commands
            .execute("editor.close", &mut state, json!({"index": 0}))
            .unwrap();
        assert_eq!(state.active_document, None);
        assert!(
            commands
                .execute("editor.close", &mut state, Value::Null)
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tool_window_commands_keep_one_window_per_anchor() {
        let commands = built_in_commands();
        let mut state = IdeState::default();
        commands
            .execute("toolwindow.toggle", &mut state, json!({"id": "problems"}))
            .unwrap();
        commands
            .execute("toolwindow.activate.find", &mut state, Value::Null)
            .unwrap();
        assert!(state.tool_windows.is_visible(ToolWindow::Find));
        assert!(!state.tool_windows.is_visible(ToolWindow::Problems));
        assert!(
            commands
                .execute("toolwindow.toggle", &mut state, json!({"id": "bogus"}))
                .is_err()
        );
    }
}
