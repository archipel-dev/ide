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

/// The type of an expression as far as this slice computes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Unknown,
    Null,
    Prim(String),
    /// A class type: FQN of the erasure (when resolved), the presentable text and the resolved
    /// type arguments (empty for a raw or non-generic type, `Unknown` for a wildcard).
    Ref {
        fqn: Option<String>,
        display: String,
        args: Vec<Ty>,
    },
    Array(Box<Ty>),
    /// A type variable in scope.
    Var(String),
}

impl Ty {
    pub fn display(&self) -> String {
        match self {
            Ty::Unknown => "?".into(),
            Ty::Null => "null".into(),
            Ty::Prim(p) => p.clone(),
            Ty::Ref { display, .. } => display.clone(),
            Ty::Array(e) => format!("{}[]", e.display()),
            Ty::Var(v) => v.clone(),
        }
    }

    pub fn string() -> Ty {
        Ty::class("java.lang.String", "String")
    }

    /// A non-generic class type.
    pub fn class(fqn: &str, display: &str) -> Ty {
        Ty::Ref {
            fqn: Some(fqn.into()),
            display: display.into(),
            args: Vec::new(),
        }
    }

    /// Replaces type variables by `map` (a variable not in it, or any variable left after
    /// `strict`, becomes `Unknown`: no inference is attempted).
    pub fn subst(&self, map: &std::collections::HashMap<String, Ty>) -> Ty {
        match self {
            Ty::Var(v) => map.get(v).cloned().unwrap_or(Ty::Unknown),
            Ty::Array(e) => match e.subst(map) {
                Ty::Unknown => Ty::Unknown,
                e => Ty::Array(Box::new(e)),
            },
            Ty::Ref { fqn, display, args } if !args.is_empty() => {
                let args: Vec<Ty> = args.iter().map(|a| a.subst(map)).collect();
                let shown = match display.find('<') {
                    Some(i) => {
                        let list: Vec<String> = args
                            .iter()
                            .map(|a| match a {
                                Ty::Unknown => "?".to_string(),
                                a => a.display(),
                            })
                            .collect();
                        format!("{}<{}>", &display[..i], list.join(", "))
                    }
                    None => display.clone(),
                };
                Ty::Ref {
                    fqn: fqn.clone(),
                    display: shown,
                    args,
                }
            }
            other => other.clone(),
        }
    }

    /// Whether a type variable is left anywhere in the type.
    pub fn has_var(&self) -> bool {
        match self {
            Ty::Var(_) => true,
            Ty::Array(e) => e.has_var(),
            Ty::Ref { args, .. } => args.iter().any(Ty::has_var),
            _ => false,
        }
    }
}
