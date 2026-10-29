//! Keywords and modifiers by position (spec 06 6.3 positions 1, 3 and 5, JAVA-CMP-011): the
//! statement keywords where a statement starts, the modifiers legal for the element and not
//! conflicting with those already written, `extends` and `implements` in a class header,
//! `throws` after a parameter list, the literals and `this` in an expression.

use super::classify::{After, Classified, Position, Slot};
use super::insert::plain_key;
use super::weigh::{Fit, Weights, tier};
use super::{Item, ItemKind};
use crate::analyze::Flow;
use crate::names::NameMatcher;
use crate::stub::ClassKind;

const PRIMITIVES: &[&str] = &[
    "boolean", "byte", "char", "double", "float", "int", "long", "short",
];

/// Keywords that take an operand or a name after them get a space.
fn needs_space(kw: &str) -> bool {
    !matches!(
        kw,
        "this" | "super" | "null" | "true" | "false" | "break" | "continue" | "return"
    )
}

struct Pool<'a> {
    matcher: NameMatcher,
    out: Vec<Item>,
    seen: std::collections::HashSet<&'a str>,
}

impl<'a> Pool<'a> {
    fn add(&mut self, kw: &'a str, tier_value: u8) {
        if !self.seen.insert(kw) {
            return;
        }
        let Some(class) = self.matcher.classify(kw) else {
            return;
        };
        let mut item = Item::blank();
        item.name = kw.to_string();
        item.kind = ItemKind::Keyword;
        item.key = plain_key(&if needs_space(kw) {
            format!("{kw} ")
        } else {
            kw.to_string()
        });
        item.class = class;
        item.sort = Weights {
            tier: tier_value,
            discouraged: false,
            deprecated: false,
            fit: Fit::Unknown,
            statics: 0,
            proximity: 0,
            name: kw,
        }
        .key();
        self.out.push(item);
    }
}

/// The primitive types as items (array creation after `new`).
pub fn primitive_types(classified: &Classified) -> Vec<Item> {
    let mut pool = Pool {
        matcher: NameMatcher::new(&classified.prefix),
        out: Vec::new(),
        seen: Default::default(),
    };
    for p in PRIMITIVES {
        pool.add(p, tier::IMPROBABLE_KEYWORD);
    }
    pool.out
}

fn has(words: &[String], w: &str) -> bool {
    words.iter().any(|x| x == w)
}

/// Modifiers still legal after `words` in a member declaration of a `kind` body.
fn member_modifiers(words: &[String], kind: Option<ClassKind>) -> Vec<&'static str> {
    let interface = matches!(kind, Some(ClassKind::Interface | ClassKind::Annotation));
    let mut out: Vec<&'static str> = if interface {
        vec![
            "public", "private", "static", "default", "abstract", "final", "strictfp", "sealed",
        ]
    } else {
        vec![
            "public",
            "protected",
            "private",
            "static",
            "final",
            "abstract",
            "native",
            "synchronized",
            "transient",
            "volatile",
            "strictfp",
            "sealed",
        ]
    };
    out.retain(|m| !has(words, m));
    let access = ["public", "protected", "private"];
    if access.iter().any(|a| has(words, a)) {
        out.retain(|m| !access.contains(m));
    }
    if has(words, "abstract") {
        out.retain(|m| {
            !matches!(
                *m,
                "final" | "static" | "private" | "native" | "synchronized" | "default"
            )
        });
    }
    if has(words, "final") || has(words, "static") || has(words, "private") {
        out.retain(|m| *m != "abstract");
    }
    if has(words, "default") {
        out.retain(|m| !matches!(*m, "static" | "abstract" | "private" | "final" | "native"));
    }
    if has(words, "static") && interface {
        out.retain(|m| *m != "default");
    }
    out
}

pub fn collect(classified: &Classified, flow: Option<&Flow>) -> Vec<Item> {
    let Position::Mixed { slot, .. } = classified.position else {
        return Vec::new();
    };
    let words = &classified.words;
    let kind = flow.and_then(|f| f.class_kind);
    let mut pool = Pool {
        matcher: NameMatcher::new(&classified.prefix),
        out: Vec::new(),
        seen: Default::default(),
    };
    let types = |pool: &mut Pool<'static>| {
        for p in PRIMITIVES {
            pool.add(p, tier::PRIMITIVE_TYPE);
        }
    };
    match slot {
        Slot::TopLevel => {
            let file = &classified.file_words;
            let declared = file
                .iter()
                .any(|w| matches!(w.as_str(), "class" | "interface" | "enum" | "record"));
            if words.is_empty() && !declared {
                if !file
                    .iter()
                    .any(|w| matches!(w.as_str(), "package" | "import"))
                {
                    pool.add("package", tier::PROBABLE_KEYWORD);
                }
                pool.add("import", tier::PROBABLE_KEYWORD);
            }
            for m in ["public", "abstract", "final", "sealed", "strictfp"] {
                if !has(words, m) {
                    pool.add(m, tier::PROBABLE_KEYWORD);
                }
            }
            if has(words, "abstract") {
                pool.out.retain(|i| i.name != "final");
            }
            if has(words, "final") {
                pool.out.retain(|i| i.name != "abstract");
            }
            for k in ["class", "interface", "enum", "record"] {
                pool.add(k, tier::PROBABLE_KEYWORD + 1);
            }
        }
        Slot::ClassBody => {
            for m in member_modifiers(words, kind) {
                pool.add(m, tier::PROBABLE_KEYWORD);
            }
            for k in ["class", "interface", "enum", "record"] {
                pool.add(k, tier::PROBABLE_KEYWORD + 1);
            }
            pool.add("void", tier::PRIMITIVE_TYPE);
            types(&mut pool);
        }
        Slot::Statement => {
            let declaring = !words.is_empty();
            if !declaring {
                for k in [
                    "if",
                    "for",
                    "while",
                    "do",
                    "switch",
                    "try",
                    "synchronized",
                    "throw",
                    "assert",
                ] {
                    pool.add(k, tier::KEYWORD);
                }
                if flow.is_none_or(|f| f.in_body) {
                    pool.add("return", tier::KEYWORD);
                }
                if let Some(f) = flow {
                    if f.in_loop || f.in_switch && !f.in_switch_expr {
                        pool.add("break", tier::KEYWORD);
                    }
                    if f.in_loop {
                        pool.add("continue", tier::KEYWORD);
                    }
                    if f.in_switch_expr {
                        pool.add("yield", tier::KEYWORD);
                    }
                    if f.at_case_position {
                        pool.add("case", tier::PROBABLE_KEYWORD);
                        pool.add("default", tier::PROBABLE_KEYWORD);
                    }
                    if f.after_if {
                        pool.add("else", tier::PROBABLE_KEYWORD);
                    }
                    if f.after_try {
                        pool.add("catch", tier::PROBABLE_KEYWORD);
                    }
                    if f.try_without_finally {
                        pool.add("finally", tier::PROBABLE_KEYWORD);
                    }
                }
                // The same from the tokens, which survive a tree the parser gave up on (a `try`
                // without its `catch` is a syntax error until it is typed).
                match classified.after {
                    After::If => pool.add("else", tier::PROBABLE_KEYWORD),
                    After::Try => {
                        pool.add("catch", tier::PROBABLE_KEYWORD);
                        pool.add("finally", tier::PROBABLE_KEYWORD);
                    }
                    After::Nothing => {}
                }
                for k in ["this", "super", "new", "null", "true", "false"] {
                    pool.add(k, tier::KEYWORD);
                }
                pool.add("final", tier::KEYWORD);
                pool.add("abstract", tier::IMPROBABLE_KEYWORD);
            } else if has(words, "final") || has(words, "abstract") {
                // After a modifier of a local declaration: types and class kinds only.
            }
            for k in ["class", "interface", "enum", "record"] {
                pool.add(k, tier::IMPROBABLE_KEYWORD);
            }
            pool.add("var", tier::PRIMITIVE_TYPE);
            types(&mut pool);
        }
        Slot::Expression => {
            for k in ["this", "super", "new", "null", "true", "false", "switch"] {
                pool.add(k, tier::KEYWORD);
            }
        }
        Slot::Parameter => {
            if !has(words, "final") {
                pool.add("final", tier::KEYWORD);
            }
            types(&mut pool);
        }
        Slot::ClassHeader => {
            let record = has(words, "record");
            let enumeration = has(words, "enum");
            let interface = has(words, "interface");
            if !record && !enumeration && !has(words, "extends") && !has(words, "implements") {
                pool.add("extends", tier::PROBABLE_KEYWORD);
            }
            if !interface && !has(words, "implements") {
                pool.add("implements", tier::PROBABLE_KEYWORD);
            }
            if has(words, "sealed") && !has(words, "permits") {
                pool.add("permits", tier::PROBABLE_KEYWORD);
            }
        }
        Slot::MethodHeader => {
            if !has(words, "throws") {
                pool.add("throws", tier::PROBABLE_KEYWORD);
            }
        }
    }
    pool.out
}
