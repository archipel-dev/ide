//! The class-name table and its matcher (JAVA-STUB-011, 017; completion 6.4.1 matching).
//!
//! `Java:Names` stores, per package, one [`NameRecord`] per named class with the summary a
//! position filter needs (kind, flags, deprecated, annotation targets), so "annotation types
//! only" or "interfaces only" never loads a stub. The records of every unit are mirrored in
//! memory as [`NameEntry`] lists keyed by unit id; a query walks the units of a resolve scope
//! (JAVA-STUB-017) and runs the [`NameMatcher`] over the short names.

use crate::stub::{ClassKind, flags};
use serde::{Deserialize, Serialize};

/// `ElementType` constants of `@Target`, as a bit mask.
pub mod target {
    pub const TYPE: u16 = 1;
    pub const FIELD: u16 = 1 << 1;
    pub const METHOD: u16 = 1 << 2;
    pub const PARAMETER: u16 = 1 << 3;
    pub const CONSTRUCTOR: u16 = 1 << 4;
    pub const LOCAL_VARIABLE: u16 = 1 << 5;
    pub const ANNOTATION_TYPE: u16 = 1 << 6;
    pub const PACKAGE: u16 = 1 << 7;
    pub const TYPE_PARAMETER: u16 = 1 << 8;
    pub const TYPE_USE: u16 = 1 << 9;
    pub const MODULE: u16 = 1 << 10;
    pub const RECORD_COMPONENT: u16 = 1 << 11;

    pub fn of_constant(name: &str) -> u16 {
        match name {
            "TYPE" => TYPE,
            "FIELD" => FIELD,
            "METHOD" => METHOD,
            "PARAMETER" => PARAMETER,
            "CONSTRUCTOR" => CONSTRUCTOR,
            "LOCAL_VARIABLE" => LOCAL_VARIABLE,
            "ANNOTATION_TYPE" => ANNOTATION_TYPE,
            "PACKAGE" => PACKAGE,
            "TYPE_PARAMETER" => TYPE_PARAMETER,
            "TYPE_USE" => TYPE_USE,
            "MODULE" => MODULE,
            "RECORD_COMPONENT" => RECORD_COMPONENT,
            _ => 0,
        }
    }
}

/// The `@Target` mask of an annotation type from its annotation texts (`None`: no `@Target`).
/// Source stubs keep the written text (`@Target({ElementType.METHOD})`), class files the
/// normalized `@java.lang.annotation.Target(METHOD, TYPE)`.
pub fn parse_targets(annotations: &[String]) -> Option<u16> {
    let text = annotations.iter().find(|a| {
        let t = a.trim_start_matches('@');
        t.starts_with("Target") || t.starts_with("java.lang.annotation.Target")
    })?;
    let args = text.split_once('(').map_or("", |(_, rest)| rest);
    let mut mask = 0u16;
    for word in args.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        mask |= target::of_constant(word);
    }
    Some(mask)
}

/// What `Java:Names` stores for one class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameRecord {
    pub fqn: String,
    pub kind: ClassKind,
    pub flags: u32,
    pub deprecated: bool,
    pub targets: Option<u16>,
}

/// One class of the in-memory name table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEntry {
    pub unit: u32,
    pub fqn: Box<str>,
    pkg_len: u32,
    short_start: u32,
    pub kind: ClassKind,
    pub flags: u32,
    pub deprecated: bool,
    pub targets: Option<u16>,
}

impl NameEntry {
    pub fn new(unit: u32, package: &str, record: NameRecord) -> Self {
        let pkg_len = package.len() as u32;
        // The short name is the last segment of the FQN (nested classes included).
        let short_start = record.fqn.rfind('.').map_or(0, |i| i + 1) as u32;
        Self {
            unit,
            fqn: record.fqn.into_boxed_str(),
            pkg_len,
            short_start,
            kind: record.kind,
            flags: record.flags,
            deprecated: record.deprecated,
            targets: record.targets,
        }
    }

    pub fn package(&self) -> &str {
        &self.fqn[..self.pkg_len as usize]
    }

    pub fn name(&self) -> &str {
        &self.fqn[self.short_start as usize..]
    }

    /// A member class (`Map.Entry`).
    pub fn is_nested(&self) -> bool {
        let start = if self.pkg_len == 0 {
            0
        } else {
            self.pkg_len as usize + 1
        };
        self.short_start as usize > start
    }

    /// FQN of the top-level class this one is declared in (itself when top-level).
    pub fn top_fqn(&self) -> &str {
        let start = if self.pkg_len == 0 {
            0
        } else {
            self.pkg_len as usize + 1
        };
        match self.fqn[start..].find('.') {
            Some(i) => &self.fqn[..start + i],
            None => &self.fqn,
        }
    }

    /// `Outer.Inner` for a member class, the simple name otherwise.
    pub fn qualified_name(&self) -> &str {
        let start = if self.pkg_len == 0 {
            0
        } else {
            self.pkg_len as usize + 1
        };
        &self.fqn[start..]
    }

    pub fn is_public(&self) -> bool {
        self.flags & flags::PUBLIC != 0
    }

    pub fn is_abstract(&self) -> bool {
        self.flags & flags::ABSTRACT != 0
            || matches!(self.kind, ClassKind::Interface | ClassKind::Annotation)
    }
}

/// Packages whose classes are implementation details of the platform, not API
/// (JAVA-CMP-025): hidden until the third press.
pub fn is_internal_package(package: &str) -> bool {
    package == "sun"
        || package.starts_with("sun.")
        || package == "com.sun"
        || package.starts_with("com.sun.")
        || package == "jdk.internal"
        || package.starts_with("jdk.internal.")
        || package.starts_with("jdk.nashorn.internal")
}

/// Packages and classes that are offered but ranked after everything else (JAVA-CMP-025,
/// `05` 5.3.1 accessibility rank "discouraged").
pub fn is_discouraged(package: &str, fqn: &str) -> bool {
    package == "org.omg" || package.starts_with("org.omg.") || fqn == "java.awt.List"
}

/// How a name matched the typed pattern, best first (JAVA-CMP-027 prefix quality).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MatchClass {
    Exact,
    /// Same case.
    Prefix,
    /// Case-insensitive prefix.
    PrefixIgnoreCase,
    /// Fragments at the humps of the name, the first at the start (`ArLi` for `ArrayList`).
    Hump,
    /// Fragments starting at a later hump (`Builder` for `StringBuilder`).
    Middle,
}

/// The one matcher of class names: camel-hump, prefix and substring tolerant, case aware
/// (an upper-case pattern letter must meet a hump start or an upper-case letter).
#[derive(Debug, Clone)]
pub struct NameMatcher {
    pattern: Vec<char>,
    lower: Vec<char>,
    text: String,
    /// Every pattern character is ASCII (the allocation-free path for ASCII names).
    ascii: bool,
}

fn is_hump(chars: &[char], i: usize) -> bool {
    let c = chars[i];
    if i == 0 {
        return true;
    }
    if !c.is_alphanumeric() {
        return false;
    }
    let p = chars[i - 1];
    if !p.is_alphanumeric() {
        return true;
    }
    if c.is_uppercase() && !p.is_uppercase() {
        return true;
    }
    if c.is_uppercase() && p.is_uppercase() && chars.get(i + 1).is_some_and(|n| n.is_lowercase()) {
        return true;
    }
    c.is_ascii_digit() != p.is_ascii_digit()
}

impl NameMatcher {
    pub fn new(pattern: &str) -> Self {
        let chars: Vec<char> = pattern.chars().collect();
        Self {
            lower: chars.iter().flat_map(|c| c.to_lowercase()).collect(),
            ascii: chars.iter().all(char::is_ascii),
            pattern: chars,
            text: pattern.to_string(),
        }
    }

    /// The first pattern character when it is ASCII: every match except a later-hump match
    /// (`Builder` in `StringBuilder`) starts with it, ignoring case, so a scan can look at the
    /// first byte of a name before it runs the matcher.
    pub fn first_ascii(&self) -> Option<u8> {
        self.pattern
            .first()
            .filter(|c| c.is_ascii())
            .map(|c| *c as u8)
    }

    pub fn pattern(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.pattern.is_empty()
    }

    /// Classifies `name`, or `None` when it does not match.
    pub fn classify(&self, name: &str) -> Option<MatchClass> {
        if self.pattern.is_empty() {
            return Some(MatchClass::Prefix);
        }
        if name == self.text {
            return Some(MatchClass::Exact);
        }
        if name.starts_with(&self.text) {
            return Some(MatchClass::Prefix);
        }
        if self.ascii && name.is_ascii() {
            return self.classify_ascii(name.as_bytes());
        }
        // Cheap rejection: every pattern letter occurs in order (ignoring case).
        {
            let mut want = self.lower.iter();
            let mut next = want.next();
            for c in name.chars().flat_map(char::to_lowercase) {
                if next == Some(&c) {
                    next = want.next();
                    if next.is_none() {
                        break;
                    }
                }
            }
            if next.is_some() {
                return None;
            }
        }
        let nlower: String = name.chars().flat_map(char::to_lowercase).collect();
        let plower: String = self.lower.iter().collect();
        if nlower.starts_with(&plower) {
            return Some(MatchClass::PrefixIgnoreCase);
        }
        let chars: Vec<char> = name.chars().collect();
        if self.hump_from(&chars, 0, 0, false, true) {
            return Some(MatchClass::Hump);
        }
        if self.hump_from(&chars, 0, 0, false, false) {
            return Some(MatchClass::Middle);
        }
        None
    }

    /// The allocation-free path for ASCII names (nearly all of them): same rules as `classify`.
    fn classify_ascii(&self, name: &[u8]) -> Option<MatchClass> {
        let p = self.text.as_bytes();
        if name.len() >= p.len() && name[..p.len()].eq_ignore_ascii_case(p) {
            return Some(MatchClass::PrefixIgnoreCase);
        }
        // Every pattern letter occurs in order (ignoring case).
        let mut want = p.iter();
        let mut next = want.next();
        for c in name {
            if next.is_some_and(|w| w.eq_ignore_ascii_case(c)) {
                next = want.next();
                if next.is_none() {
                    break;
                }
            }
        }
        if next.is_some() {
            return None;
        }
        if hump_ascii(p, name, 0, 0, false, true) {
            return Some(MatchClass::Hump);
        }
        hump_ascii(p, name, 0, 0, false, false).then_some(MatchClass::Middle)
    }

    /// Fragments of the pattern at humps of `name`; `at_start` forces the first fragment to
    /// begin at index 0.
    fn hump_from(
        &self,
        name: &[char],
        pi: usize,
        ni: usize,
        continuing: bool,
        at_start: bool,
    ) -> bool {
        if pi == self.pattern.len() {
            return true;
        }
        let p = self.pattern[pi];
        let fits = |c: char| {
            p == c
                || p.to_lowercase().eq(c.to_lowercase()) && (!p.is_uppercase() || c.is_uppercase())
        };
        if continuing
            && ni < name.len()
            && fits(name[ni])
            && self.hump_from(name, pi + 1, ni + 1, true, false)
        {
            return true;
        }
        if at_start && pi == 0 {
            return !name.is_empty()
                && p.to_lowercase().eq(name[0].to_lowercase())
                && self.hump_from(name, 1, 1, true, false);
        }
        for k in ni..name.len() {
            if is_hump(name, k)
                && p.to_lowercase().eq(name[k].to_lowercase())
                && self.hump_from(name, pi + 1, k + 1, true, false)
            {
                return true;
            }
        }
        false
    }
}

fn is_hump_ascii(n: &[u8], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let (c, p) = (n[i], n[i - 1]);
    if !c.is_ascii_alphanumeric() {
        return false;
    }
    if !p.is_ascii_alphanumeric() {
        return true;
    }
    if c.is_ascii_uppercase() && !p.is_ascii_uppercase() {
        return true;
    }
    if c.is_ascii_uppercase()
        && p.is_ascii_uppercase()
        && n.get(i + 1).is_some_and(u8::is_ascii_lowercase)
    {
        return true;
    }
    c.is_ascii_digit() != p.is_ascii_digit()
}

fn hump_ascii(p: &[u8], n: &[u8], pi: usize, ni: usize, continuing: bool, at_start: bool) -> bool {
    if pi == p.len() {
        return true;
    }
    let pc = p[pi];
    if continuing
        && ni < n.len()
        && (pc == n[ni]
            || pc.eq_ignore_ascii_case(&n[ni])
                && (!pc.is_ascii_uppercase() || n[ni].is_ascii_uppercase()))
        && hump_ascii(p, n, pi + 1, ni + 1, true, false)
    {
        return true;
    }
    if at_start && pi == 0 {
        return !n.is_empty()
            && pc.eq_ignore_ascii_case(&n[0])
            && hump_ascii(p, n, 1, 1, true, false);
    }
    for k in ni..n.len() {
        if is_hump_ascii(n, k)
            && pc.eq_ignore_ascii_case(&n[k])
            && hump_ascii(p, n, pi + 1, k + 1, true, false)
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_of_matches() {
        let m = NameMatcher::new("ArLi");
        assert_eq!(m.classify("ArrayList"), Some(MatchClass::Hump));
        assert_eq!(m.classify("ArrayDeque"), None);
        let m = NameMatcher::new("Te");
        assert_eq!(m.classify("Test"), Some(MatchClass::Prefix));
        assert_eq!(m.classify("TestInstance"), Some(MatchClass::Prefix));
        assert_eq!(m.classify("Retention"), None);
        let m = NameMatcher::new("sb");
        assert_eq!(m.classify("StringBuilder"), Some(MatchClass::Hump));
        let m = NameMatcher::new("NPE");
        assert_eq!(m.classify("NullPointerException"), Some(MatchClass::Hump));
        let m = NameMatcher::new("Builder");
        assert_eq!(m.classify("StringBuilder"), Some(MatchClass::Middle));
        let m = NameMatcher::new("List");
        assert_eq!(m.classify("List"), Some(MatchClass::Exact));
        assert_eq!(m.classify("ArrayList"), Some(MatchClass::Middle));
        assert_eq!(m.classify("Listener"), Some(MatchClass::Prefix));
        assert_eq!(
            NameMatcher::new("").classify("Anything"),
            Some(MatchClass::Prefix)
        );
    }

    #[test]
    fn humps_and_substrings() {
        let m = NameMatcher::new("AL");
        assert_eq!(m.classify("ArrayList"), Some(MatchClass::Hump));
        assert_eq!(m.classify("Alpha"), Some(MatchClass::PrefixIgnoreCase));
        assert_eq!(m.classify("Banal"), None);
        let m = NameMatcher::new("Alp");
        assert_eq!(m.classify("SomeAlpha"), Some(MatchClass::Middle));
    }

    #[test]
    fn targets_from_annotation_text() {
        let src = vec!["@Target({ElementType.METHOD, ElementType.TYPE})".to_string()];
        assert_eq!(parse_targets(&src), Some(target::METHOD | target::TYPE));
        let class = vec!["@java.lang.annotation.Target(FIELD)".to_string()];
        assert_eq!(parse_targets(&class), Some(target::FIELD));
        assert_eq!(parse_targets(&["@Deprecated".to_string()]), None);
    }

    #[test]
    fn entry_parts() {
        let e = NameEntry::new(
            1,
            "java.util",
            NameRecord {
                fqn: "java.util.Map.Entry".into(),
                kind: ClassKind::Interface,
                flags: flags::PUBLIC,
                deprecated: false,
                targets: None,
            },
        );
        assert_eq!(e.name(), "Entry");
        assert!(e.is_nested());
        assert_eq!(e.top_fqn(), "java.util.Map");
        assert_eq!(e.qualified_name(), "Map.Entry");
        assert_eq!(e.package(), "java.util");
    }
}
