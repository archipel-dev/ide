//! The member-access contributor (spec 06 6.3 position 2, JAVA-CMP-008, 016, 017, 035): after a
//! dot, the members of the qualifier's static type with their inherited ones, one row per method
//! name with its overloads merged, deprecated ones struck through, inaccessible ones left out
//! (offered, marked, from the second press), the signature and the return type as the tails.

use super::classify::Classified;
use super::insert::{call_key, plain_key};
use super::weigh::{self, Fit, Weights, tier};
use super::{CompletionInput, Item, ItemKind, probe};
use crate::analyze::DotAnswer;
use crate::analyze::members::{MemberInfo, MemberKind, MemberSet};
use crate::index::NotReady;
use crate::names::NameMatcher;
use crate::stub::flags;
use std::collections::HashMap;

pub struct MemberCompletion {
    pub items: Vec<Item>,
    pub exclusive: bool,
    /// Some supertype of the receiver did not resolve (a library is missing): the members of
    /// that supertype are not in the list.
    pub incomplete: bool,
}

pub fn complete(
    input: &CompletionInput<'_>,
    classified: &Classified,
) -> Result<Option<MemberCompletion>, NotReady> {
    let Some(probe) = probe::at(input, classified) else {
        return Ok(None);
    };
    let Some(dot) = probe.answer.dot.clone() else {
        return Ok(None);
    };
    let matcher = NameMatcher::new(&classified.prefix);
    match dot {
        // The receiver is not known (an unresolved name, an inference we do not make): the
        // language server may know; offering a guess would be wrong (no false positives).
        DotAnswer::Unknown => Ok(None),
        // `1.|`, `int x; x.|`: nothing can follow (JAVA-CMP-003).
        DotAnswer::NoMembers => Ok(Some(MemberCompletion {
            items: Vec::new(),
            exclusive: true,
            incomplete: false,
        })),
        DotAnswer::Package(package) => {
            let items = package_items(input, &package, &matcher)?;
            Ok(Some(MemberCompletion {
                items,
                exclusive: false,
                incomplete: false,
            }))
        }
        DotAnswer::Members {
            set,
            type_qualifier,
            type_context,
            enclosing,
            complete,
            ..
        } => {
            let mut items = member_items(
                &set,
                Use {
                    type_qualifier,
                    type_context,
                    instance_ok: true,
                    bias: 0,
                },
                &matcher,
            );
            if type_qualifier && !type_context {
                for (kw, text) in [("class", "class"), ("this", "this")] {
                    if kw == "this" && !enclosing {
                        continue;
                    }
                    if let Some(class) = matcher.classify(kw) {
                        let mut item = Item::blank();
                        item.name = kw.into();
                        item.kind = ItemKind::Keyword;
                        item.key = plain_key(text);
                        item.class = class;
                        item.sort = Weights {
                            tier: tier::PROBABLE_KEYWORD,
                            discouraged: false,
                            deprecated: false,
                            fit: Fit::Unknown,
                            statics: 0,
                            proximity: 0,
                            name: kw,
                        }
                        .key();
                        items.push(item);
                    }
                }
            }
            Ok(Some(MemberCompletion {
                items,
                exclusive: false,
                incomplete: !complete,
            }))
        }
    }
}

/// The classes and subpackages of `package` (JAVA-CMP-008 for a package qualifier).
fn package_items(
    input: &CompletionInput<'_>,
    package: &str,
    matcher: &NameMatcher,
) -> Result<Vec<Item>, NotReady> {
    let index = &input.project.index;
    let scope = input.project.scope_for(input.path);
    let mut items = Vec::new();
    for e in index.classes_in_package(package, &scope)? {
        if e.is_nested() || !e.is_public() && input.count < 3 {
            continue;
        }
        if crate::names::is_internal_package(package) && input.count < 3 {
            continue;
        }
        if input.settings.is_excluded(&e.fqn) {
            continue;
        }
        let Some(class) = matcher.classify(e.name()) else {
            continue;
        };
        let mut item = Item::blank();
        item.name = e.name().to_string();
        item.kind = ItemKind::Class(e.kind);
        item.key = plain_key(e.name());
        item.fqn = e.fqn.to_string();
        item.deprecated = e.deprecated;
        item.class = class;
        item.sort = weigh::class_key(false, false, e.deprecated, true, 0, e.name());
        items.push(item);
    }
    for sub in index.subpackages(package, &scope)? {
        let name = sub.rsplit('.').next().unwrap_or(&sub).to_string();
        if crate::names::is_internal_package(&sub) && input.count < 3 {
            continue;
        }
        let Some(class) = matcher.classify(&name) else {
            continue;
        };
        let mut item = Item::blank();
        item.name = name.clone();
        item.kind = ItemKind::Package;
        item.key = plain_key(&name);
        item.class = class;
        item.sort = Weights {
            tier: tier::NORMAL,
            discouraged: false,
            deprecated: false,
            fit: Fit::Unknown,
            statics: 1,
            proximity: 0,
            name: &name,
        }
        .key();
        items.push(item);
    }
    Ok(items)
}

/// `(int index, E element)`: the parameter list of one overload.
pub(crate) fn signature_of(m: &MemberInfo) -> String {
    let list: Vec<String> = m
        .params
        .iter()
        .map(|p| match &p.name {
            Some(n) => format!("{} {n}", p.ty),
            None => p.ty.clone(),
        })
        .collect();
    format!("({})", list.join(", "))
}

/// How the members of a set are reached.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Use {
    /// `Type.|`: statics and member types.
    pub type_qualifier: bool,
    /// A type position: member types only.
    pub type_context: bool,
    /// Instance members can be used (false in a static context: they are offered, ranked low).
    pub instance_ok: bool,
    /// Added to the proximity of every row: the nesting level of the class the members belong
    /// to, so the nearer scope ranks first (JAVA-CMP-027 proximity), 0 after a dot.
    pub bias: u32,
}

/// One row per method name (overloads merged), one per field, constant and member type.
pub(crate) fn member_items(set: &MemberSet, use_: Use, matcher: &NameMatcher) -> Vec<Item> {
    let Use {
        type_qualifier,
        type_context,
        instance_ok,
        bias,
    } = use_;
    let mut items = Vec::new();
    // Methods grouped by name, in order of first appearance (nearest declaration first).
    let mut groups: Vec<(&str, Vec<&MemberInfo>)> = Vec::new();
    let mut at: HashMap<&str, usize> = HashMap::new();
    for m in &set.members {
        if type_context && !matches!(m.kind, MemberKind::Type(_)) {
            continue;
        }
        if m.kind == MemberKind::Method {
            match at.get(m.name.as_str()) {
                Some(i) => groups[*i].1.push(m),
                None => {
                    at.insert(&m.name, groups.len());
                    groups.push((&m.name, vec![m]));
                }
            }
            continue;
        }
        let Some(class) = matcher.classify(&m.name) else {
            continue;
        };
        let mut row = single_row(m, class, type_qualifier, bias);
        if !instance_ok && !m.is_static || !m.accessible {
            row.sort = demote(&row.sort);
        }
        items.push(row);
    }
    for (name, overloads) in groups {
        let Some(class) = matcher.classify(name) else {
            continue;
        };
        let mut row = method_row(name, &overloads, class, type_qualifier, bias);
        if !instance_ok && !overloads.iter().any(|m| m.is_static)
            || !overloads.iter().any(|m| m.accessible)
        {
            row.sort = demote(&row.sort);
        }
        items.push(row);
    }
    items
}

/// A member that a static context cannot use is not hidden but ranked low (spec 06 6.3 filters).
fn demote(sort: &str) -> String {
    // The tier moves to the rest tier.
    format!("{:02}{}", tier::REST, &sort[2..])
}

fn single_row(
    m: &MemberInfo,
    class: crate::names::MatchClass,
    type_qualifier: bool,
    bias: u32,
) -> Item {
    let mut item = Item::blank();
    item.name = m.name.clone();
    item.deprecated = m.deprecated;
    item.class = class;
    item.ty = m.ty.clone();
    item.key = plain_key(&m.name);
    let (tier_value, statics) = match m.kind {
        MemberKind::Type(kind) => {
            item.kind = ItemKind::Class(kind);
            item.fqn = format!("{}.{}", m.owner.qname(), m.name);
            (tier::NORMAL, 0)
        }
        MemberKind::EnumConstant => {
            item.kind = ItemKind::Constant;
            (tier::VARIABLE, u8::from(!type_qualifier))
        }
        _ => {
            item.kind = ItemKind::Field;
            (
                // Instance fields are variables; a static field is a normal member (spec 06
                // 6.5 item 5), so `"x".|` does not lead with `String.CASE_INSENSITIVE_ORDER`.
                if m.is_static {
                    tier::NORMAL
                } else {
                    tier::VARIABLE
                },
                if type_qualifier {
                    u8::from(!m.is_static)
                } else {
                    u8::from(m.is_static)
                },
            )
        }
    };
    let mut w = weights(m, tier_value, statics, &m.name);
    w.proximity += bias;
    item.sort = w.key();
    if !m.accessible {
        item.note = visibility_note(m.flags);
    }
    item
}

fn visibility_note(f: u32) -> String {
    if f & flags::PRIVATE != 0 {
        "private".into()
    } else if f & flags::PROTECTED != 0 {
        "protected".into()
    } else {
        "package-private".into()
    }
}

fn weights<'a>(m: &MemberInfo, tier_value: u8, statics: u8, name: &'a str) -> Weights<'a> {
    // Nearer declarations first; the members of Object after the domain members; the ones the
    // place cannot reach last (JAVA-CMP-027 proximity).
    let mut proximity = (m.depth as u32).min(9_999);
    if m.from_object {
        proximity += 50_000;
    }
    if !m.accessible {
        proximity += 30_000;
    }
    Weights {
        tier: tier_value,
        discouraged: false,
        deprecated: m.deprecated,
        fit: Fit::Unknown,
        statics,
        proximity,
        name,
    }
}

fn method_row(
    name: &str,
    overloads: &[&MemberInfo],
    class: crate::names::MatchClass,
    type_qualifier: bool,
    bias: u32,
) -> Item {
    // JAVA-CMP-016, 035: one row per method name; the representative overload is the first of
    // the nearest declaration.
    let first = overloads[0];
    let mut item = Item::blank();
    item.name = name.to_string();
    item.kind = ItemKind::Method;
    item.class = class;
    item.signature = signature_of(first);
    item.ty = first.ty.clone();
    // JAVA-CMP-017: a merged row is struck through when every overload is deprecated.
    item.deprecated = overloads.iter().all(|m| m.deprecated);
    let no_args = overloads.len() == 1 && first.params.is_empty();
    item.key = call_key(name, !no_args);
    let accessible = overloads.iter().any(|m| m.accessible);
    let statics = if type_qualifier {
        u8::from(!overloads.iter().any(|m| m.is_static))
    } else {
        u8::from(overloads.iter().all(|m| m.is_static))
    };
    let mut w = weights(first, tier::NORMAL, statics, name);
    w.proximity += bias;
    w.deprecated = item.deprecated;
    if accessible && !first.accessible {
        // An accessible overload decides for the merged row.
        w.proximity = w.proximity.saturating_sub(30_000);
    }
    item.sort = w.key();
    if !accessible {
        item.note = visibility_note(first.flags);
    }
    item
}
