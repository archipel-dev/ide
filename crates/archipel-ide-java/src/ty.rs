//! Type text helpers and the small type model used by resolve and applicability
//! (the slice of spec 03 that spec 05 section 5.2.3 needs: erasure-level assignability,
//! boxing, widening; inference is out of this slice and unknown types are permissive).

/// A type text split into its erased base, the raw type-argument text and the array depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parts {
    pub base: String,
    pub args: Option<String>,
    pub dims: usize,
}

pub fn split(text: &str) -> Parts {
    let mut t = text.trim();
    let mut dims = 0;
    loop {
        if let Some(r) = t.strip_suffix("[]") {
            dims += 1;
            t = r.trim_end();
        } else if let Some(r) = t.strip_suffix("...") {
            dims += 1;
            t = r.trim_end();
        } else {
            break;
        }
    }
    // Annotations on types are dropped.
    let (base, args) = match t.find('<') {
        Some(i) if t.ends_with('>') => (t[..i].trim(), Some(t[i + 1..t.len() - 1].to_string())),
        Some(i) => (t[..i].trim(), None),
        None => (t, None),
    };
    Parts {
        base: base.replace(' ', ""),
        args,
        dims,
    }
}

/// Splits `a, b<c, d>, e` at top-level commas.
pub fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    for ch in args.chars() {
        match ch {
            '<' => {
                depth += 1;
                cur.push(ch);
            }
            '>' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

pub fn is_primitive(name: &str) -> bool {
    matches!(
        name,
        "int" | "long" | "short" | "byte" | "char" | "boolean" | "float" | "double"
    )
}

pub fn boxed_name(prim: &str) -> &'static str {
    match prim {
        "int" => "java.lang.Integer",
        "long" => "java.lang.Long",
        "short" => "java.lang.Short",
        "byte" => "java.lang.Byte",
        "char" => "java.lang.Character",
        "boolean" => "java.lang.Boolean",
        "float" => "java.lang.Float",
        "double" => "java.lang.Double",
        _ => "java.lang.Object",
    }
}

pub fn unboxed_name(fqn: &str) -> Option<&'static str> {
    Some(match fqn {
        "java.lang.Integer" => "int",
        "java.lang.Long" => "long",
        "java.lang.Short" => "short",
        "java.lang.Byte" => "byte",
        "java.lang.Character" => "char",
        "java.lang.Boolean" => "boolean",
        "java.lang.Float" => "float",
        "java.lang.Double" => "double",
        _ => return None,
    })
}

/// Primitive widening (JLS 5.1.2), identity included.
pub fn prim_widens(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    let order = |p: &str| match p {
        "byte" => 1,
        "short" => 2,
        "int" => 3,
        "long" => 4,
        "float" => 5,
        "double" => 6,
        _ => 0,
    };
    match (from, to) {
        ("char", t) => order(t) >= 3,
        (f, "char") => {
            let _ = f;
            false
        }
        ("boolean", _) | (_, "boolean") => false,
        (f, t) => order(f) != 0 && order(f) < order(t),
    }
}

/// Short presentable form: qualifiers removed from every name (`java.util.List<java.lang.String>`
/// becomes `List<String>`), spacing normalized to `, `.
pub fn display(text: &str) -> String {
    let mut out = String::new();
    let mut ident = String::new();
    let flush = |ident: &mut String, out: &mut String| {
        if !ident.is_empty() {
            out.push_str(ident.rsplit('.').next().unwrap_or(ident));
            ident.clear();
        }
    };
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '$' || ch == '.' {
            ident.push(ch);
        } else {
            // `...` of varargs must not be eaten as a qualifier.
            flush(&mut ident, &mut out);
            if ch == ',' {
                out.push_str(", ");
            } else if !ch.is_whitespace() {
                out.push(ch);
            } else if out.ends_with("extends") || out.ends_with("super") {
                out.push(' ');
            }
        }
    }
    flush(&mut ident, &mut out);
    out.replace(",  ", ", ")
        .replace("?extends", "? extends")
        .replace("?super", "? super")
}

/// The type of an expression or declaration as far as this slice computes it (spec 03 3.1,
/// JAVA-TYP-001..003): a closed, structural model. Class types carry the FQN of their erasure
/// and their type arguments; the presentable text is derived, never stored, so substitution
/// (JAVA-TYP-013) rebuilds types instead of editing strings. Subtyping, inference and capture
/// (M5) extend the operations on this type; they do not replace it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    /// Not known (unresolved, not inferred yet): no member, no error is derived from it
    /// (JAVA-TYP-032).
    Unknown,
    Null,
    Prim(String),
    /// A class type: FQN of the erasure (`None` when the written name did not resolve), the
    /// simple name for presentation and the type arguments (empty for a raw or non-generic type).
    Ref {
        fqn: Option<String>,
        name: String,
        args: Vec<Ty>,
    },
    Array(Box<Ty>),
    /// A type variable in scope; its bounds are looked up at the declaration that introduced it.
    Var(String),
    /// A wildcard type argument; it only occurs inside [`Ty::Ref::args`].
    Wildcard(Wild),
}

/// The bound of a wildcard type argument (JAVA-TYP-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wild {
    /// `?`
    Any,
    /// `? extends T`
    Extends(Box<Ty>),
    /// `? super T`
    Super(Box<Ty>),
}

impl Ty {
    /// The canonical presentable form: simple names, `, ` between arguments (JAVA-TYP-003).
    pub fn display(&self) -> String {
        match self {
            Ty::Unknown => "?".into(),
            Ty::Null => "null".into(),
            Ty::Prim(p) => p.clone(),
            Ty::Ref { name, args, .. } => {
                if args.is_empty() {
                    name.clone()
                } else {
                    let list: Vec<String> = args.iter().map(Ty::display).collect();
                    format!("{name}<{}>", list.join(", "))
                }
            }
            Ty::Array(e) => format!("{}[]", e.display()),
            Ty::Var(v) => v.clone(),
            Ty::Wildcard(Wild::Any) => "?".into(),
            Ty::Wildcard(Wild::Extends(b)) => format!("? extends {}", b.display()),
            Ty::Wildcard(Wild::Super(b)) => format!("? super {}", b.display()),
        }
    }

    pub fn string() -> Ty {
        Ty::class("java.lang.String", "String")
    }

    /// A non-generic class type.
    pub fn class(fqn: &str, name: &str) -> Ty {
        Ty::Ref {
            fqn: Some(fqn.into()),
            name: name.into(),
            args: Vec::new(),
        }
    }

    /// A class type with arguments.
    pub fn generic(fqn: &str, name: &str, args: Vec<Ty>) -> Ty {
        Ty::Ref {
            fqn: Some(fqn.into()),
            name: name.into(),
            args,
        }
    }

    /// The FQN of the class type (not of arrays, primitives or variables).
    pub fn class_fqn(&self) -> Option<&str> {
        match self {
            Ty::Ref { fqn: Some(f), .. } => Some(f),
            _ => None,
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Ty::Unknown)
    }

    /// The type a *read* through this type argument yields: the upper bound of a wildcard
    /// (`? extends T` is `T`); `?` and `? super T` have no useful static bound here and are
    /// unknown (the capture would be the declared bound of the parameter, M5).
    pub fn read_type(&self) -> Ty {
        match self {
            Ty::Wildcard(Wild::Extends(b)) => (**b).clone(),
            Ty::Wildcard(_) => Ty::Unknown,
            t => t.clone(),
        }
    }

    /// Replaces type variables by `map` (a variable not in it becomes `Unknown`: no inference
    /// is attempted). A wildcard that replaces a variable in a *top-level* position is read
    /// through its bound ([`Ty::read_type`]).
    pub fn subst(&self, map: &std::collections::HashMap<String, Ty>) -> Ty {
        match self {
            Ty::Var(v) => map.get(v).map_or(Ty::Unknown, Ty::read_type),
            Ty::Array(e) => match e.subst(map) {
                Ty::Unknown => Ty::Unknown,
                e => Ty::Array(Box::new(e)),
            },
            Ty::Ref { fqn, name, args } if !args.is_empty() => Ty::Ref {
                fqn: fqn.clone(),
                name: name.clone(),
                args: args.iter().map(|a| a.subst_arg(map)).collect(),
            },
            other => other.clone(),
        }
    }

    /// Like [`Ty::subst`] for presentation: a variable that `map` does not name stays as it is
    /// (`<T> T[] toArray(T[])` keeps its `T`).
    pub fn subst_partial(&self, map: &std::collections::HashMap<String, Ty>) -> Ty {
        match self {
            Ty::Var(v) => map.get(v).map_or_else(
                || self.clone(),
                |t| match t {
                    Ty::Unknown => self.clone(),
                    t => t.clone(),
                },
            ),
            Ty::Array(e) => Ty::Array(Box::new(e.subst_partial(map))),
            Ty::Ref { fqn, name, args } => Ty::Ref {
                fqn: fqn.clone(),
                name: name.clone(),
                args: args.iter().map(|a| a.subst_partial(map)).collect(),
            },
            Ty::Wildcard(Wild::Extends(b)) => {
                Ty::Wildcard(Wild::Extends(Box::new(b.subst_partial(map))))
            }
            Ty::Wildcard(Wild::Super(b)) => {
                Ty::Wildcard(Wild::Super(Box::new(b.subst_partial(map))))
            }
            t => t.clone(),
        }
    }

    /// Substitution inside type arguments, where a wildcard stays a wildcard.
    fn subst_arg(&self, map: &std::collections::HashMap<String, Ty>) -> Ty {
        match self {
            Ty::Var(v) => map.get(v).cloned().unwrap_or(Ty::Unknown),
            Ty::Wildcard(Wild::Extends(b)) => match b.subst(map) {
                Ty::Unknown => Ty::Wildcard(Wild::Any),
                b => Ty::Wildcard(Wild::Extends(Box::new(b))),
            },
            Ty::Wildcard(Wild::Super(b)) => match b.subst(map) {
                Ty::Unknown => Ty::Wildcard(Wild::Any),
                b => Ty::Wildcard(Wild::Super(Box::new(b))),
            },
            t => t.subst(map),
        }
    }

    /// Whether a type variable is left anywhere in the type.
    pub fn has_var(&self) -> bool {
        match self {
            Ty::Var(_) => true,
            Ty::Array(e) => e.has_var(),
            Ty::Ref { args, .. } => args.iter().any(Ty::has_var),
            Ty::Wildcard(Wild::Extends(b) | Wild::Super(b)) => b.has_var(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn list_of(arg: Ty) -> Ty {
        Ty::generic("java.util.List", "List", vec![arg])
    }

    /// JAVA-TYP-003: the presentable form is derived from the structure.
    #[test]
    fn display_is_canonical() {
        let map = Ty::generic(
            "java.util.Map",
            "Map",
            vec![
                Ty::string(),
                list_of(Ty::Wildcard(Wild::Extends(Box::new(Ty::class(
                    "java.lang.Number",
                    "Number",
                ))))),
            ],
        );
        assert_eq!(map.display(), "Map<String, List<? extends Number>>");
        assert_eq!(
            Ty::Array(Box::new(Ty::Prim("int".into()))).display(),
            "int[]"
        );
        assert_eq!(list_of(Ty::Wildcard(Wild::Any)).display(), "List<?>");
        assert_eq!(
            list_of(Ty::Wildcard(Wild::Super(Box::new(Ty::string())))).display(),
            "List<? super String>"
        );
    }

    /// JAVA-TYP-013: variables are replaced through the receiver's arguments; a variable the
    /// receiver does not bind is unknown for reads and stays for presentation.
    #[test]
    fn substitution_binds_variables() {
        let mut map = HashMap::new();
        map.insert("E".to_string(), Ty::string());
        let ret = list_of(Ty::Var("E".into()));
        assert_eq!(ret.subst(&map).display(), "List<String>");
        assert_eq!(Ty::Var("E".into()).subst(&map), Ty::string());
        assert_eq!(Ty::Var("T".into()).subst(&map), Ty::Unknown);
        assert_eq!(Ty::Var("T".into()).subst_partial(&map), Ty::Var("T".into()));
        assert_eq!(
            Ty::Array(Box::new(Ty::Var("E".into()))).subst(&map),
            Ty::Array(Box::new(Ty::string()))
        );
        assert!(ret.has_var());
        assert!(!ret.subst(&map).has_var());
    }

    /// A read through a wildcard argument is its upper bound; `?` and `? super T` are unknown
    /// (no capture yet), and a wildcard stays a wildcard inside a nested argument.
    #[test]
    fn reads_through_wildcards_use_the_upper_bound() {
        let number = Ty::class("java.lang.Number", "Number");
        let mut map = HashMap::new();
        map.insert(
            "E".to_string(),
            Ty::Wildcard(Wild::Extends(Box::new(number.clone()))),
        );
        assert_eq!(Ty::Var("E".into()).subst(&map), number);
        assert_eq!(
            list_of(Ty::Var("E".into())).subst(&map).display(),
            "List<? extends Number>"
        );
        map.insert("E".to_string(), Ty::Wildcard(Wild::Any));
        assert_eq!(Ty::Var("E".into()).subst(&map), Ty::Unknown);
        map.insert("E".to_string(), Ty::Wildcard(Wild::Super(Box::new(number))));
        assert_eq!(Ty::Var("E".into()).subst(&map), Ty::Unknown);
    }
}
