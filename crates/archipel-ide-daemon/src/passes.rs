//! The built-in passes: `Syntax`, `General` (annotators), `LocalInspections`,
//! `ExternalAnnotators`, `ExternalDiagnostics` (language servers), `IdentifierHighlighter` and `LineMarkers`.
//! (`ProblemFiles` is a model over the markup, see [`crate::problem_files`].)

use crate::external::{ExternalDiagnostics, ExternalDiagnosticsPass};
use crate::inspection::engine::{LocalRun, run_local, to_highlight, walk_range};
use crate::inspection::profile::EffectiveProfile;
use crate::scheduler::{PassSpec, Triggers};
use archipel_ide_analysis::daemon::{
    AnnotateCx, AnnotationHolder, Annotator, DynExternalAnnotator, Pass, PassCx, PassId, PassOutput,
};
use archipel_ide_analysis::highlight::ProblemGroup;
use archipel_ide_analysis::inspect::{InspectionSuppressor, LocalInspection};
use archipel_ide_analysis::{
    Cancelled, FileId, HighlightInfo, HighlightKind, Severity, SyntaxNode, TokenSet, ToolId,
};
use archipel_ide_language::{Registration, ServiceRegistry};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

pub const SYNTAX: PassId = PassId("syntax");
pub const GENERAL: PassId = PassId("general");
pub const LOCAL_INSPECTIONS: PassId = PassId("local-inspections");
pub const EXTERNAL_ANNOTATORS: PassId = PassId("external-annotators");
pub const EXTERNAL_DIAGNOSTICS: PassId = PassId("external-diagnostics");
pub const IDENTIFIER_HIGHLIGHTER: PassId = PassId("identifier-highlighter");
pub const LINE_MARKERS: PassId = PassId("line-markers");

/// State shared by the built-in passes and the daemon.
#[derive(Debug, Default)]
pub struct Shared {
    pub profile: RwLock<Arc<EffectiveProfile>>,
    pub carets: Mutex<HashMap<FileId, u32>>,
    /// What external providers (language servers) published; read by
    /// [`crate::external::ExternalDiagnosticsPass`].
    pub external: Arc<ExternalDiagnostics>,
    /// Language ids whose native analysis wins a dedupe against a language server although the
    /// language itself has not graduated (`LanguageDef::tier`): a user-facing setting that
    /// follows the differential test (`docs/analysis/java-differential.md`).
    pub native_wins: RwLock<std::collections::HashSet<String>>,
}

/// The scheduling properties of the built-in passes (inline, triggers); any
/// other registered pass is an ordinary content pass.
pub fn spec_of(pass: &dyn Pass) -> PassSpec {
    let id = pass.id();
    let mut spec = PassSpec::new(id)
        .after(pass.after())
        .debounce(pass.debounce());
    if pass.visible_range_first() {
        spec = spec.visible_first();
    }
    match id {
        SYNTAX => spec.inline().listens(Triggers::SYNTAX),
        EXTERNAL_ANNOTATORS => spec.listens(Triggers::CONTENT.union(Triggers::DIAGNOSTICS)),
        EXTERNAL_DIAGNOSTICS => spec.listens(Triggers::DIAGNOSTICS),
        IDENTIFIER_HIGHLIGHTER => spec.listens(
            Triggers::CARET
                .union(Triggers::SYNTAX)
                .union(Triggers::INDEX),
        ),
        LINE_MARKERS => spec.listens(Triggers::CONTENT.union(Triggers::VISIBLE)),
        _ => spec,
    }
}

fn whole(cx: &PassCx<'_>) -> Range<u32> {
    0..cx.syntax.text().len() as u32
}

fn out(cx: &PassCx<'_>, tool: ToolId, range: Range<u32>, infos: Vec<HighlightInfo>) -> PassOutput {
    PassOutput {
        file: cx.file,
        rev: cx.syntax.rev(),
        tool,
        range,
        infos,
    }
}

/// The range an output replaces: the visible slice, or everything.
fn slice(cx: &PassCx<'_>) -> Range<u32> {
    if cx.visible {
        cx.range.clone()
    } else {
        0..u32::MAX
    }
}

/// A one-character highlight at `at` (the character at or before it).
fn one_char(text: &str, at: usize) -> Range<u32> {
    let at = at.min(text.len());
    let mut start = at;
    if start == text.len() && start > 0 {
        start -= 1;
    }
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    let end = text[start..]
        .chars()
        .next()
        .map_or(start, |c| start + c.len_utf8());
    start as u32..end as u32
}

// -------------------------------------------------------------------- syntax

/// Syntax errors of the parse (ERROR and MISSING nodes), one character wide.
#[derive(Debug, Default)]
pub struct SyntaxPass;

impl Pass for SyntaxPass {
    fn id(&self) -> PassId {
        SYNTAX
    }
    fn after(&self) -> &[PassId] {
        &[]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        false
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        let text = cx.syntax.text();
        let infos = cx
            .syntax
            .errors()
            .iter()
            .map(|error| {
                let range = one_char(text, error.range.start as usize);
                HighlightInfo::new(
                    range,
                    Severity::ERROR,
                    HighlightKind::Error,
                    ToolId::native("syntax"),
                )
                .message(error.message.clone())
            })
            .collect();
        Ok(out(cx, ToolId::native("syntax"), 0..u32::MAX, infos))
    }
}

// ------------------------------------------------------------------- general

/// Runs the registered [`Annotator`]s over the elements they are interested
/// in, visible range first.
#[derive(Debug, Default)]
pub struct GeneralPass;

impl Pass for GeneralPass {
    fn id(&self) -> PassId {
        GENERAL
    }
    fn after(&self) -> &[PassId] {
        &[SYNTAX]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        true
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        let lang = cx.syntax.lang();
        let dumb = cx.snap.is_dumb();
        let annotators: Vec<Arc<dyn Annotator>> =
            cx.snap.registry().all_available(lang.language_id(), dumb);
        let tool = ToolId::native("general");
        let mut holder = AnnotationHolder::new(tool.clone());
        if !annotators.is_empty() {
            let acx = AnnotateCx {
                snap: cx.snap,
                syntax: cx.syntax,
                cancel: cx.cancel,
                tier: lang.tier,
                dumb,
            };
            let range = if cx.visible { Some(&cx.range) } else { None };
            walk_range(cx.syntax.root(), range, cx.cancel, |node| {
                for annotator in &annotators {
                    if annotator.interesting().contains(node.element_type()) {
                        annotator.annotate(node, &mut holder, &acx)?;
                    }
                }
                Ok(())
            })?;
        }
        let slice = slice(cx);
        let infos = holder
            .into_infos()
            .into_iter()
            .filter(|info| slice.start <= info.range.start && info.range.start < slice.end)
            .collect();
        Ok(out(cx, tool, slice, infos))
    }
}

// ------------------------------------------------------ local inspections

/// Runs the enabled local inspections (one dispatch table, one tree walk).
#[derive(Debug)]
pub struct LocalInspectionsPass {
    shared: Arc<Shared>,
}

impl LocalInspectionsPass {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

impl Pass for LocalInspectionsPass {
    fn id(&self) -> PassId {
        LOCAL_INSPECTIONS
    }
    fn after(&self) -> &[PassId] {
        &[GENERAL]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        true
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        let lang = cx.syntax.lang();
        let dumb = cx.snap.is_dumb();
        let profile = self.shared.profile.read().clone();
        let path = cx
            .snap
            .path(cx.file)
            .map(|p| p.to_owned())
            .unwrap_or_default();
        let tools: Vec<_> = cx
            .snap
            .registry()
            .all_available::<dyn LocalInspection>(lang.language_id(), dumb)
            .iter()
            .filter(|tool| lang.tier >= tool.min_tier() && (tool.dumb_aware() || !dumb))
            .filter_map(|tool| profile.configure(tool, &path))
            .collect();
        let suppressors: Vec<Arc<dyn InspectionSuppressor>> =
            cx.snap.registry().all(lang.language_id());
        let run = LocalRun {
            snap: cx.snap,
            file: cx.file,
            syntax: cx.syntax,
            cancel: cx.cancel,
            on_the_fly: true,
        };
        let range = cx.visible.then(|| cx.range.clone());
        let found = run_local(&run, &tools, &suppressors, range)?;
        let infos = found.iter().map(to_highlight).collect();
        Ok(out(
            cx,
            ToolId::native("local-inspections"),
            slice(cx),
            infos,
        ))
    }
}

// ------------------------------------------------------ external annotators

/// Runs the registered external annotators (slow tools: LSP, linters).
#[derive(Debug, Default)]
pub struct ExternalAnnotatorsPass;

impl Pass for ExternalAnnotatorsPass {
    fn id(&self) -> PassId {
        EXTERNAL_ANNOTATORS
    }
    fn after(&self) -> &[PassId] {
        &[LOCAL_INSPECTIONS]
    }
    fn dumb_aware(&self) -> bool {
        false
    }
    fn visible_range_first(&self) -> bool {
        false
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        let lang = cx.syntax.lang();
        let annotators: Vec<Arc<dyn DynExternalAnnotator>> = cx
            .snap
            .registry()
            .all_available(lang.language_id(), cx.snap.is_dumb());
        let tool = ToolId::native("external");
        let mut holder = AnnotationHolder::new(tool.clone());
        for annotator in annotators {
            cx.cancel.check()?;
            let Some(info) = annotator.collect_dyn(cx.snap, cx.file) else {
                continue;
            };
            cx.cancel.check()?;
            if let Some(result) = annotator.run_dyn(info, cx.cancel) {
                cx.cancel.check()?;
                annotator.apply_dyn(cx.file, cx.syntax.rev(), result, &mut holder);
            }
        }
        let len = whole(cx).end;
        let infos = holder
            .into_infos()
            .into_iter()
            .filter(|info| info.range.end <= len)
            .collect();
        Ok(out(cx, tool, 0..u32::MAX, infos))
    }
}

// ----------------------------------------------------- identifier highlight

/// Highlights the occurrences of the identifier under the caret. Without
/// resolve (G0) occurrences are matched by kind and text; a language with
/// file-local resolve can register its own pass under the same id.
#[derive(Debug)]
pub struct IdentifierHighlighterPass {
    shared: Arc<Shared>,
}

impl IdentifierHighlighterPass {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self { shared }
    }
}

impl Pass for IdentifierHighlighterPass {
    fn id(&self) -> PassId {
        IDENTIFIER_HIGHLIGHTER
    }
    fn after(&self) -> &[PassId] {
        &[]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        false
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        let tool = ToolId::native("identifier-highlighter");
        let caret = self.shared.carets.lock().get(&cx.file).copied();
        let Some(caret) = caret else {
            return Ok(out(cx, tool, 0..u32::MAX, Vec::new()));
        };
        let identifiers: &TokenSet = &cx.syntax.lang().token_set_table().identifiers;
        let root = cx.syntax.root();
        let at = root
            .find_leaf_at(caret)
            .filter(|n| identifiers.contains(n.element_type()))
            .or_else(|| {
                // Caret right behind the identifier.
                caret
                    .checked_sub(1)
                    .and_then(|c| root.find_leaf_at(c))
                    .filter(|n| identifiers.contains(n.element_type()))
            });
        let Some(target) = at else {
            return Ok(out(cx, tool, 0..u32::MAX, Vec::new()));
        };
        let (kind, name) = (target.element_type(), target.text());
        let mut infos = Vec::new();
        walk_range(root, None, cx.cancel, |node: SyntaxNode<'_>| {
            if node.element_type() == kind && node.first_child().is_none() && node.text() == name {
                infos.push(HighlightInfo::new(
                    node.range(),
                    Severity::TEXT_ATTRIBUTES,
                    HighlightKind::Semantic("IDENTIFIER_UNDER_CARET"),
                    tool.clone(),
                ));
            }
            Ok(())
        })?;
        if infos.len() < 2 {
            infos.clear();
        }
        Ok(out(cx, tool, 0..u32::MAX, infos))
    }
}

// -------------------------------------------------------------- line markers

/// A gutter marker (overrides, recursion, runnable, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineMarker {
    /// Range the marker belongs to; the gutter icon sits on its first line.
    pub range: Range<u32>,
    /// Stable kind, e.g. `overrides`, `recursion`, `run` (icon selector).
    pub kind: &'static str,
    pub tooltip: String,
}

/// Provides gutter markers for elements of the kinds it is interested in.
pub trait LineMarkerProvider: Send + Sync {
    fn interesting(&self) -> &TokenSet;
    fn markers(&self, node: SyntaxNode<'_>, out: &mut Vec<LineMarker>);
}

/// Collects the markers of every registered [`LineMarkerProvider`].
#[derive(Debug, Default)]
pub struct LineMarkersPass;

impl Pass for LineMarkersPass {
    fn id(&self) -> PassId {
        LINE_MARKERS
    }
    fn after(&self) -> &[PassId] {
        &[GENERAL]
    }
    fn dumb_aware(&self) -> bool {
        true
    }
    fn visible_range_first(&self) -> bool {
        true
    }
    fn collect(&self, cx: &PassCx<'_>) -> Result<PassOutput, Cancelled> {
        let lang = cx.syntax.lang();
        let providers: Vec<Arc<dyn LineMarkerProvider>> = cx
            .snap
            .registry()
            .all_available(lang.language_id(), cx.snap.is_dumb());
        let tool = ToolId::native("line-markers");
        let slice = slice(cx);
        let mut markers = Vec::new();
        if !providers.is_empty() {
            let range = cx.visible.then_some(&cx.range);
            walk_range(cx.syntax.root(), range, cx.cancel, |node| {
                for provider in &providers {
                    if provider.interesting().contains(node.element_type()) {
                        provider.markers(node, &mut markers);
                    }
                }
                Ok(())
            })?;
        }
        let infos = markers
            .into_iter()
            .filter(|m| slice.start <= m.range.start && m.range.start < slice.end)
            .map(|m| {
                let mut info = HighlightInfo::new(
                    m.range,
                    Severity::TEXT_ATTRIBUTES,
                    HighlightKind::Semantic("LINE_MARKER"),
                    tool.clone(),
                )
                .message(m.tooltip);
                info.group = Some(ProblemGroup(format!("line-marker:{}", m.kind).into()));
                info
            })
            .collect();
        Ok(out(cx, tool, slice, infos))
    }
}

/// Registers the built-in passes for every language.
pub fn register_passes(registry: &mut ServiceRegistry, shared: &Arc<Shared>) {
    let any = |id: &'static str, dumb: bool| Registration::any(id).dumb_aware(dumb);
    registry.register::<dyn Pass>(any("syntax", true), Arc::new(SyntaxPass));
    registry.register::<dyn Pass>(any("general", true), Arc::new(GeneralPass));
    registry.register::<dyn Pass>(
        any("local-inspections", true),
        Arc::new(LocalInspectionsPass::new(shared.clone())),
    );
    registry.register::<dyn Pass>(
        any("external-annotators", false),
        Arc::new(ExternalAnnotatorsPass),
    );
    registry.register::<dyn Pass>(
        any("external-diagnostics", true),
        Arc::new(ExternalDiagnosticsPass::new(shared.external.clone())),
    );
    registry.register::<dyn Pass>(
        any("identifier-highlighter", true),
        Arc::new(IdentifierHighlighterPass::new(shared.clone())),
    );
    registry.register::<dyn Pass>(any("line-markers", true), Arc::new(LineMarkersPass));
}
