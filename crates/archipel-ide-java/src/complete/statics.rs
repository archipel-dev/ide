//! Static members of any class on the second press (spec 06 6.4.3, JAVA-CMP-022): in an
//! unqualified identifier position with a typed prefix, count >= 2 adds the static methods and
//! fields of every class of the resolve scope whose name matches, from the static member name
//! index. Each class contributes once per name (overloads merged); a row reads `Class.member`
//! with the package as its tail. Choosing it writes `Class.member(` and imports the class, or,
//! when the class is already imported statically or the caret is inside it, just `member(`.

use super::classify::Classified;
use super::insert::escape;
use super::probe::Probe;
use super::weigh::{Fit, Weights, tier};
use super::{CompletionInput, Insertion, Item, ItemKind};
use crate::env::Env;
use crate::import_fix::{Place, known_element};
use crate::index::{ClassHit, NotReady};
use crate::names::{NameMatcher, is_internal_package};
use crate::ty;
use std::collections::HashSet;

/// Names and members taken from the index before grouping (bounds the stubs decoded).
const MAX_NAMES: usize = 30;
const MAX_HITS: usize = 300;
/// Rows offered at most.
const MAX_ROWS: usize = 100;

fn params_of(m: &crate::stub::MethodStub) -> String {
    let list: Vec<String> = m
        .params
        .iter()
        .map(|p| {
            let t = ty::display(&p.ty);
            let t = if p.varargs {
                t.strip_suffix("[]")
                    .map_or(t.clone(), |b| format!("{b}..."))
            } else {
                t
            };
            if p.generated {
                t
            } else {
                format!("{t} {}", p.name)
            }
        })
        .collect();
    format!("({})", list.join(", "))
}

pub fn collect(
    input: &CompletionInput<'_>,
    classified: &Classified,
    probe: &Probe,
) -> Result<Vec<Item>, NotReady> {
    let project = input.project;
    let index = &project.index;
    let scope = project.scope_for(input.path);
    let matcher = NameMatcher::new(&classified.prefix);
    let hits = index.static_members_matching(&scope, &matcher, MAX_NAMES, MAX_HITS)?;
    let cur_unit = index.unit_for_path(input.path);
    let env = Env::new(index, &scope, None);
    let (package, top) = match &probe.answer.scope {
        Some(s) => (s.facts.place.package.clone(), s.facts.place.top.clone()),
        None => (String::new(), None),
    };
    let stub = super::class_names::file_stub(input);
    let place = Place {
        path: input.path.to_path_buf(),
        package: package.clone(),
        top_class: top.clone(),
        module: scope.module.clone(),
        test: scope.test,
        imports: stub.imports.clone(),
    };
    // Members already reachable unqualified: no second row for them.
    let imported: HashSet<(String, String)> = probe
        .answer
        .scope
        .iter()
        .flat_map(|s| s.imported.iter())
        .flat_map(|(_, set)| {
            set.members
                .iter()
                .map(|m| (m.owner.qname().to_string(), m.name.clone()))
        })
        .collect();
    // One group per (class, name), in order of discovery (nearest match first).
    let mut groups: Vec<(ClassHit, String, bool, Vec<usize>)> = Vec::new();
    for h in hits {
        if Some(h.class.unit) == cur_unit && cur_unit.is_some() {
            continue; // the live file's own members come from its fresh stub
        }
        let class = h.class.class();
        let name = if h.field {
            class.fields[h.index].name.clone()
        } else {
            class.methods[h.index].name.clone()
        };
        let fqn = class.qname.clone();
        if h.class.package().is_empty() && !package.is_empty() {
            continue; // default-package members are not offered outside it
        }
        if input.count < 3 && is_internal_package(h.class.package()) {
            continue;
        }
        if input.settings.is_excluded(&fqn) || input.settings.is_excluded(&format!("{fqn}.{name}"))
        {
            continue;
        }
        if imported.contains(&(fqn.clone(), name.clone())) {
            continue;
        }
        let member_flags = if h.field {
            class.fields[h.index].flags
        } else {
            class.methods[h.index].flags
        };
        if !env.accessible(&h.class, member_flags, &package, top.as_deref(), false) {
            continue;
        }
        match groups
            .iter_mut()
            .find(|(c, n, f, _)| c.qname() == fqn && *n == name && *f == h.field)
        {
            Some((_, _, _, members)) => members.push(h.index),
            None => {
                if groups.len() < MAX_ROWS {
                    groups.push((h.class.clone(), name, h.field, vec![h.index]));
                }
            }
        }
    }
    let mut items = Vec::new();
    for (class_hit, name, field, members) in groups {
        let Some(mc) = matcher.classify(&name) else {
            continue;
        };
        let class = class_hit.class();
        let fqn = class.qname.clone();
        let rel = fqn
            .strip_prefix(&format!("{}.", class_hit.package()))
            .unwrap_or(&fqn)
            .to_string();
        let mut item = Item::blank();
        item.name = name.clone();
        item.display = Some(format!("{rel}.{name}"));
        item.filter = Some(name.clone());
        item.visible = false;
        item.tail = class_hit.package().to_string();
        let first = members[0];
        let (deprecated, mode) = if field {
            let f = &class.fields[first];
            item.kind = if f.enum_constant {
                ItemKind::Constant
            } else {
                ItemKind::Field
            };
            item.ty = ty::display(&f.ty);
            (f.deprecated, 'f')
        } else {
            let m = &class.methods[first];
            item.kind = ItemKind::Method;
            item.signature = params_of(m);
            item.ty = m.ret.as_deref().map(ty::display).unwrap_or_default();
            let no_args = members.len() == 1 && m.params.is_empty();
            (m.deprecated, if no_args { 'z' } else { 'c' })
        };
        let deprecated = deprecated
            || class.deprecated
            || members
                .iter()
                .all(|i| !field && class.methods[*i].deprecated);
        item.deprecated = deprecated;
        item.class = mc;
        item.key = format!("\u{1}s{fqn}\u{2}{name}\u{2}{mode}");
        // Not yet imported: the class-name tier; popular classes (`Math`, `Collections`,
        // `Assertions` in tests) first, then the nearer classpath entries (JAVA-CMP-027).
        let known = known_element(&fqn, &place, true).clamp(-1, 9);
        let proximity = (9 - known) as u32 * 1000 + class_hit.rank.min(999);
        item.sort = Weights {
            tier: tier::CLASS_NAME,
            discouraged: false,
            deprecated,
            fit: Fit::Unknown,
            statics: 0,
            proximity,
            name: &item.display.clone().unwrap_or_default(),
        }
        .key();
        items.push(item);
    }
    Ok(items)
}

/// Writes `Class.member(` and the class import, or `member(` when the member is reachable
/// without the class (a static import of the class, or the caret inside it).
pub fn insertion(
    input: &CompletionInput<'_>,
    payload: &str,
) -> Result<Option<Insertion>, NotReady> {
    let mut parts = payload.split('\u{2}');
    let (Some(fqn), Some(member), Some(mode)) = (parts.next(), parts.next(), parts.next()) else {
        return Ok(None);
    };
    let stub = super::class_names::file_stub(input);
    let imported_statically = stub.imports.iter().any(|i| {
        i.is_static
            && (i.on_demand && i.name == fqn || !i.on_demand && i.name == format!("{fqn}.{member}"))
    });
    let inside = enclosing_classes(input).iter().any(|c| c == fqn);
    let (prefix_text, edits) = if imported_statically || inside {
        (String::new(), Vec::new())
    } else {
        match super::class_names::insertion(input, fqn)? {
            Some(ins) => (format!("{}.", ins.text), ins.edits),
            None => return Ok(None),
        }
    };
    let rest = &input.text[input.offset.min(input.text.len())..];
    let tail: usize = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '$'))
        .map(char::len_utf8)
        .sum();
    let has_call = rest[tail..]
        .trim_start_matches([' ', '\t'])
        .starts_with('(');
    let (suffix, snippet) = match mode {
        "f" => (String::new(), false),
        _ if has_call => (String::new(), false),
        "z" => ("()".to_string(), false),
        _ => ("($0)".to_string(), true),
    };
    let text = format!(
        "{}{}{suffix}",
        escape_if(snippet, &prefix_text),
        escape_if(snippet, member)
    );
    Ok(Some(Insertion {
        text,
        edits,
        snippet,
    }))
}

fn escape_if(snippet: bool, text: &str) -> String {
    if snippet {
        escape(text)
    } else {
        text.to_string()
    }
}

/// The qualified names of the classes that contain the caret.
fn enclosing_classes(input: &CompletionInput<'_>) -> Vec<String> {
    let Some(tree) = crate::source::parse(input.text) else {
        return Vec::new();
    };
    let package = crate::source::build_from_tree(&tree, input.text).package;
    let mut out = Vec::new();
    let mut node = tree
        .root_node()
        .descendant_for_byte_range(input.offset.saturating_sub(1), input.offset);
    let mut names: Vec<String> = Vec::new();
    while let Some(n) = node {
        if crate::source::is_class_kind(n.kind())
            && let Some(name) = n.child_by_field_name("name")
        {
            names.push(input.text[name.byte_range()].to_string());
        }
        node = n.parent();
    }
    names.reverse();
    let mut qualified = package;
    for n in names {
        qualified = if qualified.is_empty() {
            n
        } else {
            format!("{qualified}.{n}")
        };
        out.push(qualified.clone());
    }
    out
}
