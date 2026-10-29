//! The tree the caret sits in. The typed prefix (and the rest of the identifier under the caret)
//! is replaced by a placeholder identifier, the broken statement is repaired (`repair`), and the
//! resolver answers what completion asks at the placeholder: the qualifier of a dot, the
//! variables and members in scope, the expected type and the statement context.

use super::CompletionInput;
use super::classify::{Classified, Position, Slot};
use crate::analyze::{self, Flow, PLACEHOLDER, ProbeAnswer};
use crate::env::{CurrentFile, Env};
use crate::index::NotReady;
use crate::ty::Ty;
use std::collections::HashSet;

/// The tree cache slot of the probe text (the original text uses [`ORIGINAL`]).
pub(super) const PROBE: u8 = 1;
/// The tree cache slot of the buffer as it is.
pub(super) const ORIGINAL: u8 = 0;

/// The resolver's answers at the caret.
pub struct Probe {
    pub answer: ProbeAnswer,
}

fn is_ident_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// What the analysis of the placeholder needs: set by the position.
fn wants(position: Position) -> (bool, bool, bool) {
    match position {
        Position::Qualified => (true, false, false),
        Position::New => (false, false, true),
        Position::Mixed {
            slot: Slot::Statement | Slot::Expression,
            ..
        } => (false, true, true),
        // Declarations and headers only need the statement context.
        _ => (false, false, false),
    }
}

/// Probes the tree at the caret. `None`: the tree around the caret is too broken to say anything
/// reliable (the caller then offers nothing native from it).
pub fn at(input: &CompletionInput<'_>, classified: &Classified) -> Option<Probe> {
    let rest = &input.text[input.offset.min(input.text.len())..];
    let tail: usize = rest
        .chars()
        .take_while(|c| is_ident_part(*c))
        .map(char::len_utf8)
        .sum();
    let mut text = String::with_capacity(input.text.len() + PLACEHOLDER.len());
    text.push_str(&input.text[..classified.prefix_start]);
    text.push_str(PLACEHOLDER);
    // `Type.<A>name`: the explicit type arguments belong to a call, which needs its parentheses
    // to parse.
    if classified.position == Position::Qualified
        && input.text[..classified.prefix_start]
            .trim_end()
            .ends_with('>')
        && !rest[tail..]
            .trim_start_matches([' ', '\t'])
            .starts_with('(')
    {
        text.push_str("()");
    }
    text.push_str(&input.text[input.offset + tail..]);
    // The previous probe of this file is the base of the parse: a keystroke changes a few bytes.
    let base = input.project.index.remembered_tree(input.path, PROBE);
    let prep = crate::repair::prepare_incremental(
        &text,
        base.as_ref().map(|(t, tree)| (t.as_str(), tree)),
    )?;
    input
        .project
        .index
        .remember_tree(input.path, PROBE, prep.text.clone(), prep.tree.clone());
    let at = prep.text.find(PLACEHOLDER)?;
    let ph = at..at + PLACEHOLDER.len();
    let project = input.project;
    let scope = project.scope_for(input.path);
    let stub = crate::source::build_from_tree(&prep.tree, &prep.text);
    let current = CurrentFile::new(project.index.unit_for_path(input.path), stub);
    let env = Env::new(&project.index, &scope, Some(&current));
    let (dot, vars, expected) = wants(classified.position);
    let answer = analyze::probe_answer(
        &env,
        &prep.text,
        &prep.tree,
        &current,
        ph,
        analyze::ProbeWants {
            dot,
            scope: vars,
            expected,
            include_inaccessible: input.count >= 2,
        },
    )
    .ok()??;
    Some(Probe { answer })
}

impl Probe {
    /// The classes that suit the expected type better than the others: the expected class and
    /// its inheritors (`List<String> l = new Arr|` prefers `ArrayList`, JAVA-CMP-027).
    pub fn suitable_classes(
        &self,
        input: &CompletionInput<'_>,
    ) -> Result<HashSet<String>, NotReady> {
        let mut out = HashSet::new();
        let scope = input.project.scope_for(input.path);
        for t in &self.answer.expected {
            let Ty::Ref { fqn: Some(f), .. } = t else {
                continue;
            };
            if f == "java.lang.Object" {
                continue;
            }
            let short = f.rsplit('.').next().unwrap_or(f);
            out.insert(f.clone());
            out.extend(input.project.index.inheritor_closure(short, &scope)?);
        }
        Ok(out)
    }

    pub fn flow(&self) -> &Flow {
        &self.answer.flow
    }
}
