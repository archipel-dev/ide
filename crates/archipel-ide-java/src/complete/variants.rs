//! Reference variants of an unqualified identifier (spec 06 6.3 position 1, JAVA-CMP-008,
//! JAVA-RES-001): local variables and parameters, fields and methods of the enclosing classes
//! with the inherited ones, enum constants, and the members brought in by static imports.
//! They come from the resolver's own scope walk, so completion and errors cannot disagree.

use super::classify::Classified;
use super::insert::plain_key;
use super::members::{Use, member_items};
use super::probe::Probe;
use super::weigh::{Fit, Weights, tier};
use super::{CompletionInput, Item, ItemKind};
use crate::index::NotReady;
use crate::names::NameMatcher;
use crate::ty::Ty;
use std::collections::HashSet;

/// Whether the type `t` is the expected type or one of its subtypes (the platform ranking's
/// expected-type match, from the inheritor closure of the expected classes).
fn fits(t: &Ty, expected: &[Ty], suitable: &HashSet<String>) -> Fit {
    if expected.is_empty() {
        return Fit::Unknown;
    }
    match t {
        Ty::Unknown | Ty::Var(_) => Fit::Unknown,
        Ty::Prim(p) => {
            let matches = expected.iter().any(|e| match e {
                Ty::Prim(q) => p == q,
                Ty::Ref { fqn: Some(f), .. } => crate::ty::boxed_name(p) == f,
                _ => false,
            });
            if matches {
                Fit::Matches
            } else {
                Fit::DoesNotMatch
            }
        }
        Ty::Ref { fqn: Some(f), .. } => {
            if suitable.contains(f) {
                return Fit::Matches;
            }
            let boxed = crate::ty::unboxed_name(f);
            let matches = expected.iter().any(|e| match e {
                Ty::Prim(q) => boxed == Some(q.as_str()),
                Ty::Ref { fqn: Some(g), .. } => g == "java.lang.Object" || g == f,
                _ => false,
            });
            if matches {
                Fit::Matches
            } else {
                Fit::DoesNotMatch
            }
        }
        Ty::Array(_) => {
            let matches = expected.iter().any(|e| {
                matches!(e, Ty::Array(_))
                    || matches!(e, Ty::Ref { fqn: Some(g), .. } if g == "java.lang.Object")
            });
            if matches {
                Fit::Matches
            } else {
                Fit::DoesNotMatch
            }
        }
        _ => Fit::Unknown,
    }
}

pub fn collect(
    input: &CompletionInput<'_>,
    classified: &Classified,
    probe: &Probe,
) -> Result<Vec<Item>, NotReady> {
    let Some(scope) = &probe.answer.scope else {
        return Ok(Vec::new());
    };
    let matcher = NameMatcher::new(&classified.prefix);
    let expected = &probe.answer.expected;
    let suitable = probe.suitable_classes(input)?;
    let mut items: Vec<Item> = Vec::new();
    let mut taken: HashSet<String> = HashSet::new();
    // Locals and parameters, innermost first.
    for (rank, l) in scope.facts.locals.iter().enumerate() {
        taken.insert(l.name.clone());
        let Some(class) = matcher.classify(&l.name) else {
            continue;
        };
        let fit = fits(&l.ty, expected, &suitable);
        let mut item = Item::blank();
        item.name = l.name.clone();
        item.kind = if l.is_param {
            ItemKind::Parameter
        } else {
            ItemKind::Local
        };
        item.key = plain_key(&l.name);
        item.class = class;
        item.ty = if l.ty.is_unknown() {
            String::new()
        } else {
            l.ty.display()
        };
        item.sort = Weights {
            tier: if fit == Fit::Matches {
                tier::VARIABLE_OF_EXPECTED_TYPE
            } else {
                tier::VARIABLE
            },
            discouraged: false,
            deprecated: false,
            fit,
            statics: 0,
            proximity: rank as u32,
            name: &l.name,
        }
        .key();
        items.push(item);
    }
    // Members of the enclosing classes, innermost first; a nearer declaration hides the rest.
    let mut method_names: HashSet<String> = HashSet::new();
    for (depth, level) in scope.levels.iter().enumerate() {
        let rows = member_items(
            &level.set,
            Use {
                type_qualifier: false,
                type_context: false,
                instance_ok: level.instance_ok,
                // After the locals, the nearer class first.
                bias: 1_000 * (depth as u32 + 1),
            },
            &matcher,
        );
        let mut level_methods: HashSet<String> = HashSet::new();
        for mut row in rows {
            match row.kind {
                ItemKind::Method => {
                    if method_names.contains(&row.name) {
                        continue;
                    }
                    level_methods.insert(row.name.clone());
                }
                ItemKind::Field | ItemKind::Constant => {
                    if taken.contains(&row.name) {
                        continue;
                    }
                }
                // Member types are class names: the class-name contributor owns them.
                ItemKind::Class(_) => continue,
                _ => {}
            }
            if matches!(row.kind, ItemKind::Field | ItemKind::Constant) {
                taken.insert(row.name.clone());
            }
            // Expected-type tiers for fields.
            if matches!(row.kind, ItemKind::Field | ItemKind::Constant)
                && !expected.is_empty()
                && let Some(m) = level
                    .set
                    .members
                    .iter()
                    .find(|m| m.name == row.name && !m.is_method())
                && fits(&m.ty_model, expected, &suitable) == Fit::Matches
                && !row.sort.starts_with("20")
            {
                row.sort = format!("{:02}{}", tier::VARIABLE_OF_EXPECTED_TYPE, &row.sort[2..]);
            }
            items.push(row);
        }
        method_names.extend(level_methods);
    }
    // Statically imported members (JAVA-RES-021).
    for (_, set) in &scope.imported {
        let rows = member_items(
            set,
            Use {
                type_qualifier: true,
                type_context: false,
                instance_ok: true,
                bias: 1_000 * (scope.levels.len() as u32 + 1),
            },
            &matcher,
        );
        for row in rows {
            if matches!(row.kind, ItemKind::Class(_)) {
                continue;
            }
            if row.kind == ItemKind::Method && method_names.contains(&row.name) {
                continue;
            }
            if matches!(row.kind, ItemKind::Field | ItemKind::Constant) && taken.contains(&row.name)
            {
                continue;
            }
            items.push(row);
        }
    }
    Ok(items)
}
