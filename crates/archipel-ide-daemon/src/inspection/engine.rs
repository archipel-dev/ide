//! The local-inspection engine: one dispatch table per file, one tree walk
//! that feeds every enabled tool (the `InspectionVisitorOptimizer` idea),
//! suppression and conversion of problems into highlights.

use archipel_ide_analysis::FileId;
use archipel_ide_analysis::highlight::FixDescriptor;
use archipel_ide_analysis::inspect::{
    FileCx, InspectCx, InspectionOptions, InspectionSuppressor, Level, LocalInspection,
    OptionValue, ProblemDescriptor, ProblemHighlightType, ProblemsHolder,
};
use archipel_ide_analysis::{
    CancelToken, Cancelled, HighlightInfo, HighlightKind, Severity, Snapshot, SyntaxNode,
    SyntaxSnapshot, ToolId,
};
use smallvec::SmallVec;
use std::ops::Range;
use std::sync::Arc;

/// An enabled tool with the level and options its profile gives it.
#[derive(Clone)]
pub struct ToolConfig {
    pub tool: Arc<dyn LocalInspection>,
    pub level: Level,
    pub options: InspectionOptions,
}

impl std::fmt::Debug for ToolConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolConfig")
            .field("id", &self.tool.id())
            .field("level", &self.level)
            .finish()
    }
}

/// A problem with the tool that found it.
#[derive(Debug, Clone)]
pub struct Found {
    pub tool: &'static str,
    pub level: Level,
    pub problem: ProblemDescriptor,
}

/// Typed access to option values with defaults.
pub trait OptionsExt {
    fn int(&self, id: &str, default: i64) -> i64;
    fn flag(&self, id: &str, default: bool) -> bool;
    fn text(&self, id: &str, default: &str) -> String;
}

impl OptionsExt for InspectionOptions {
    fn int(&self, id: &str, default: i64) -> i64 {
        match self.0.get(id) {
            Some(OptionValue::Int(v)) => *v,
            _ => default,
        }
    }

    fn flag(&self, id: &str, default: bool) -> bool {
        match self.0.get(id) {
            Some(OptionValue::Bool(v)) => *v,
            _ => default,
        }
    }

    fn text(&self, id: &str, default: &str) -> String {
        match self.0.get(id) {
            Some(OptionValue::Text(v)) => v.clone(),
            _ => default.to_owned(),
        }
    }
}

/// Element kind -> tools interested in it, plus the whole-file tools.
#[derive(Debug, Default)]
pub struct Dispatch {
    by_kind: Vec<SmallVec<[u16; 4]>>,
    whole_file: Vec<u16>,
}

impl Dispatch {
    /// `kinds` = number of element types of the file's grammar.
    pub fn build(tools: &[ToolConfig], kinds: usize) -> Self {
        let mut dispatch = Self {
            by_kind: vec![SmallVec::new(); kinds],
            whole_file: Vec::new(),
        };
        for (index, config) in tools.iter().enumerate() {
            let index = index as u16;
            match config.tool.interesting() {
                None => dispatch.whole_file.push(index),
                Some(set) => {
                    for ty in set.iter() {
                        if let Some(slot) = dispatch.by_kind.get_mut(ty.0 as usize) {
                            slot.push(index);
                        }
                    }
                }
            }
        }
        dispatch
    }

    pub fn tools_for(&self, kind: usize) -> &[u16] {
        self.by_kind.get(kind).map_or(&[], |s| s.as_slice())
    }

    pub fn whole_file(&self) -> &[u16] {
        &self.whole_file
    }
}

/// Inputs of one engine run.
pub struct LocalRun<'a> {
    pub snap: &'a Snapshot,
    pub file: FileId,
    pub syntax: &'a SyntaxSnapshot,
    pub cancel: &'a CancelToken,
    pub on_the_fly: bool,
}

fn intersects(node: &Range<u32>, range: &Range<u32>) -> bool {
    (node.start < range.end && node.end > range.start)
        || (node.is_empty() && range.contains(&node.start))
}

/// Pre-order walk of the nodes intersecting `range` (whole tree for `None`).
pub fn walk_range<'s>(
    root: SyntaxNode<'s>,
    range: Option<&Range<u32>>,
    cancel: &CancelToken,
    mut visit: impl FnMut(SyntaxNode<'s>) -> Result<(), Cancelled>,
) -> Result<(), Cancelled> {
    let mut checkpoint = cancel.checkpoint();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        checkpoint.tick()?;
        if let Some(range) = range
            && !intersects(&node.range(), range)
        {
            continue;
        }
        visit(node)?;
        let children: SmallVec<[SyntaxNode<'s>; 16]> = node.children().collect();
        stack.extend(children.into_iter().rev());
    }
    Ok(())
}

/// Runs `tools` over the file. With `range = Some(r)` only element tools run,
/// over the nodes intersecting `r`, and only problems *starting* in `r` are
/// kept (so the slice replaced in the markup is exactly this range); with
/// `None` the whole file is checked, whole-file tools included.
pub fn run_local(
    run: &LocalRun<'_>,
    tools: &[ToolConfig],
    suppressors: &[Arc<dyn InspectionSuppressor>],
    range: Option<Range<u32>>,
) -> Result<Vec<Found>, Cancelled> {
    if tools.is_empty() {
        return Ok(Vec::new());
    }
    let lang = run.syntax.lang();
    let dispatch = Dispatch::build(tools, lang.element_types().len());
    let mut holders: Vec<ProblemsHolder> = tools
        .iter()
        .map(|_| ProblemsHolder::new(run.on_the_fly))
        .collect();
    walk_range(run.syntax.root(), range.as_ref(), run.cancel, |node| {
        for &index in dispatch.tools_for(node.element_type().0 as usize) {
            let config = &tools[index as usize];
            let cx = InspectCx {
                snap: run.snap,
                syntax: run.syntax,
                cancel: run.cancel,
                options: &config.options,
                on_the_fly: run.on_the_fly,
            };
            config
                .tool
                .check_element(node, &mut holders[index as usize], &cx)?;
        }
        Ok(())
    })?;
    if range.is_none() {
        for &index in dispatch.whole_file() {
            run.cancel.check()?;
            let config = &tools[index as usize];
            let cx = FileCx {
                snap: run.snap,
                file: run.file,
                syntax: run.syntax,
                cancel: run.cancel,
                options: &config.options,
                on_the_fly: run.on_the_fly,
            };
            config.tool.check_file(&cx, &mut holders[index as usize])?;
        }
    }
    let len = run.syntax.text().len() as u32;
    let mut found = Vec::new();
    for (config, holder) in tools.iter().zip(holders) {
        for mut problem in holder.into_problems() {
            run.cancel.check()?;
            if problem.on_the_fly_only && !run.on_the_fly {
                continue;
            }
            problem.range = clamp_range(run.syntax.text(), problem.range);
            if problem.range.end > len {
                continue;
            }
            if let Some(range) = &range
                && !(range.start <= problem.range.start && problem.range.start < range.end)
            {
                continue;
            }
            let node = run.syntax.covering(problem.range.clone());
            if suppressors
                .iter()
                .any(|s| s.is_suppressed(node, config.tool.id()))
            {
                continue;
            }
            found.push(Found {
                tool: config.tool.id(),
                level: config.level,
                problem,
            });
        }
    }
    found.sort_by_key(|f| (f.problem.range.start, f.problem.range.end));
    Ok(found)
}

/// Clamps a tool-reported range into `text` and snaps it to characters.
pub fn clamp_range(text: &str, range: Range<u32>) -> Range<u32> {
    let len = text.len();
    let mut start = (range.start as usize).min(len);
    while !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (range.end as usize).clamp(start, len);
    while !text.is_char_boundary(end) {
        end += 1;
    }
    start as u32..end as u32
}

/// The severity a profile level gives a highlight.
pub fn severity_of(level: Level) -> Severity {
    match level {
        Level::Error => Severity::ERROR,
        Level::Warning => Severity::WARNING,
        Level::WeakWarning => Severity::WEAK_WARNING,
        Level::Information => Severity::INFORMATION,
        Level::TextAttributes => Severity::TEXT_ATTRIBUTES,
    }
}

fn kind_of_level(level: Level) -> HighlightKind {
    match level {
        Level::Error => HighlightKind::Error,
        Level::Warning => HighlightKind::Warning,
        Level::WeakWarning => HighlightKind::WeakWarning,
        Level::Information => HighlightKind::Information,
        Level::TextAttributes => HighlightKind::TextAttributes("INSPECTION"),
    }
}

/// Converts a found problem into a highlight owned by `native:<tool id>`.
pub fn to_highlight(found: &Found) -> HighlightInfo {
    let (severity, kind) = match found.problem.highlight {
        ProblemHighlightType::Error => (Severity::ERROR, HighlightKind::Error),
        ProblemHighlightType::Warning => (Severity::WARNING, HighlightKind::Warning),
        ProblemHighlightType::WeakWarning => (Severity::WEAK_WARNING, HighlightKind::WeakWarning),
        ProblemHighlightType::Information => (Severity::INFORMATION, HighlightKind::Information),
        ProblemHighlightType::LikeUnusedSymbol => (severity_of(found.level), HighlightKind::Unused),
        ProblemHighlightType::LikeDeprecated => {
            (severity_of(found.level), HighlightKind::Deprecated)
        }
        ProblemHighlightType::LikeMarkedForRemoval => {
            (severity_of(found.level), HighlightKind::MarkedForRemoval)
        }
        ProblemHighlightType::LikeUnknownSymbol => {
            (severity_of(found.level), HighlightKind::Unresolved)
        }
        ProblemHighlightType::GenericErrorOrWarning | ProblemHighlightType::PossibleProblem => {
            (severity_of(found.level), kind_of_level(found.level))
        }
    };
    let mut info = HighlightInfo::new(
        found.problem.range.clone(),
        severity,
        kind,
        ToolId::native(found.tool),
    )
    .message(found.problem.message.as_str());
    for fix in &found.problem.fixes {
        info.fixes.push(FixDescriptor {
            family: fix.family().into(),
            text: fix.name().into(),
            action: Some(fix.clone()),
        });
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamping_keeps_ranges_inside_the_text_on_boundaries() {
        assert_eq!(clamp_range("aä", 0..99), 0..3);
        assert_eq!(clamp_range("aä", 2..2), 1..3);
        assert_eq!(clamp_range("abc", 5..9), 3..3);
    }
}
