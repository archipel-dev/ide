//! The error-tolerant position classifier of completion (spec 06 section 6.3, JAVA-CMP-009;
//! plan 2.7). It reads the *tokens before the caret* with a small lexer and a frame stack
//! (braces and parentheses with their kind), so it works in a broken tree where the parser
//! produced an ERROR node around a half-typed `@Te` or `new ArrayLi`.

use crate::names::target;

/// What the caret position asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// After `@`: annotation types, filtered by the annotated element when it is known.
    Annotation { target: Option<u16> },
    /// After `new `: instantiable classes.
    New,
    /// A position where only these types are legal (JAVA-CMP-009).
    Type(TypeFilter),
    /// A declaration or expression position that also takes members, keywords and locals:
    /// the class names join the other variants (the language server's, until the main
    /// contributor is native).
    Mixed { declaration: bool },
    /// Nothing to complete natively (comment, string, number, qualified name, import).
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeFilter {
    /// `class A extends |`: non-final classes.
    ExtendsClass,
    /// `interface A extends |`, `class A implements |`: interfaces.
    Interface,
    /// `throws |`, `catch (|`: `Throwable` subtypes.
    Throwable,
    /// `permits |`, `instanceof |`, bounds: any class, interface, enum or record.
    Any,
    /// A type argument: any class, interface, enum or record.
    Argument,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BraceKind {
    ClassBody,
    Block,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParenKind {
    /// Parameters of a method, constructor or record header.
    Params,
    AnnotationArgs,
    Catch,
    /// `if`, `while`, `for`, `switch`, `synchronized`, try resources, calls.
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classified {
    pub position: Position,
    /// Byte offset where the identifier prefix starts.
    pub prefix_start: usize,
    pub prefix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ident,
    Number,
    Punct(char),
    Arrow,
    Literal,
}

#[derive(Debug, Clone, Copy)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

#[derive(Debug, PartialEq, Eq)]
enum End {
    Code,
    Comment,
    Javadoc,
    Literal,
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_ident_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Tokens of `src` and the lexical state at its end.
fn lex(src: &str) -> (Vec<Tok>, End) {
    let b = src.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            match src[i..].find('\n') {
                Some(n) => {
                    i += n + 1;
                    continue;
                }
                None => return (toks, End::Comment),
            }
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            let javadoc = b.get(i + 2) == Some(&b'*') && b.get(i + 3) != Some(&b'/');
            match src[i + 2..].find("*/") {
                Some(n) => {
                    i += n + 4;
                    continue;
                }
                None => {
                    return (toks, if javadoc { End::Javadoc } else { End::Comment });
                }
            }
        }
        if c == b'"' {
            let start = i;
            if src[i..].starts_with("\"\"\"") {
                match src[i + 3..].find("\"\"\"") {
                    Some(n) => {
                        i += n + 6;
                        toks.push(Tok {
                            kind: Kind::Literal,
                            start,
                            end: i,
                        });
                        continue;
                    }
                    None => return (toks, End::Literal),
                }
            }
            i += 1;
            let mut closed = false;
            while i < b.len() {
                match b[i] {
                    b'\\' => i += 2,
                    b'"' => {
                        i += 1;
                        closed = true;
                        break;
                    }
                    b'\n' => break,
                    _ => i += 1,
                }
            }
            if !closed && i >= b.len() {
                return (toks, End::Literal);
            }
            toks.push(Tok {
                kind: Kind::Literal,
                start,
                end: i.min(b.len()),
            });
            continue;
        }
        if c == b'\'' {
            let start = i;
            i += 1;
            let mut closed = false;
            while i < b.len() {
                match b[i] {
                    b'\\' => i += 2,
                    b'\'' => {
                        i += 1;
                        closed = true;
                        break;
                    }
                    b'\n' => break,
                    _ => i += 1,
                }
            }
            if !closed && i >= b.len() {
                return (toks, End::Literal);
            }
            toks.push(Tok {
                kind: Kind::Literal,
                start,
                end: i.min(b.len()),
            });
            continue;
        }
        let ch = src[i..].chars().next().unwrap_or(' ');
        if is_ident_start(ch) {
            let start = i;
            i += ch.len_utf8();
            while let Some(n) = src[i..].chars().next() {
                if is_ident_part(n) {
                    i += n.len_utf8();
                } else {
                    break;
                }
            }
            toks.push(Tok {
                kind: Kind::Ident,
                start,
                end: i,
            });
            continue;
        }
        if ch.is_ascii_digit() {
            let start = i;
            while let Some(n) = src[i..].chars().next() {
                if n.is_alphanumeric()
                    || n == '_'
                    || n == '.' && src[i + 1..].starts_with(|d: char| d.is_ascii_digit())
                {
                    i += n.len_utf8();
                } else {
                    break;
                }
            }
            toks.push(Tok {
                kind: Kind::Number,
                start,
                end: i,
            });
            continue;
        }
        if c == b'-' && b.get(i + 1) == Some(&b'>') {
            toks.push(Tok {
                kind: Kind::Arrow,
                start: i,
                end: i + 2,
            });
            i += 2;
            continue;
        }
        toks.push(Tok {
            kind: Kind::Punct(ch),
            start: i,
            end: i + ch.len_utf8(),
        });
        i += ch.len_utf8();
    }
    (toks, End::Code)
}

#[derive(Debug, Clone, Copy)]
enum Frame {
    Brace(BraceKind),
    Paren(ParenKind),
}

const MODIFIERS: &[&str] = &[
    "public",
    "private",
    "protected",
    "static",
    "final",
    "abstract",
    "synchronized",
    "native",
    "transient",
    "volatile",
    "default",
    "sealed",
    "strictfp",
];

struct Scan<'a> {
    src: &'a str,
    toks: &'a [Tok],
}

impl<'a> Scan<'a> {
    fn text(&self, i: usize) -> &'a str {
        &self.src[self.toks[i].start..self.toks[i].end]
    }

    fn is_kw(&self, i: usize, kw: &str) -> bool {
        self.toks[i].kind == Kind::Ident && self.text(i) == kw
    }

    fn is_punct(&self, i: usize, c: char) -> bool {
        self.toks[i].kind == Kind::Punct(c)
    }

    /// Kind of the brace opened by token `at`, given the header tokens `from..at`.
    fn brace_kind(&self, from: usize, at: usize) -> BraceKind {
        let mut depth = 0i32;
        let mut has_new = false;
        for i in from..at {
            match self.toks[i].kind {
                Kind::Punct('(') => depth += 1,
                Kind::Punct(')') => depth -= 1,
                Kind::Ident if depth == 0 => {
                    let t = self.text(i);
                    let after_dot = i > from && self.is_punct(i - 1, '.');
                    if matches!(t, "class" | "interface" | "enum") && !after_dot {
                        return BraceKind::ClassBody;
                    }
                    if t == "record"
                        && !after_dot
                        && self.toks.get(i + 1).is_some_and(|n| n.kind == Kind::Ident)
                        && i + 1 < at
                    {
                        return BraceKind::ClassBody;
                    }
                    if t == "new" {
                        has_new = true;
                    }
                }
                _ => {}
            }
        }
        if at > from {
            let last = at - 1;
            if has_new && self.is_punct(last, ')') {
                return BraceKind::ClassBody;
            }
            if matches!(
                self.toks[last].kind,
                Kind::Punct('=') | Kind::Punct(',') | Kind::Punct('(') | Kind::Punct('{')
            ) {
                return BraceKind::Other;
            }
        }
        BraceKind::Block
    }

    /// Kind of the parenthesis opened by token `at` with the header starting at `from`.
    fn paren_kind(&self, from: usize, at: usize, top: Option<Frame>) -> ParenKind {
        if at > 0 {
            let p = at - 1;
            if self.toks[p].kind == Kind::Ident {
                let t = self.text(p);
                if t == "catch" {
                    return ParenKind::Catch;
                }
                if matches!(
                    t,
                    "if" | "for" | "while" | "switch" | "synchronized" | "try"
                ) {
                    return ParenKind::Other;
                }
                // `@Name(` and `@pkg.Name(`
                let mut q = p;
                while q >= 2 && self.is_punct(q - 1, '.') && self.toks[q - 2].kind == Kind::Ident {
                    q -= 2;
                }
                if q >= 1 && self.is_punct(q - 1, '@') {
                    return ParenKind::AnnotationArgs;
                }
            }
        }
        // A method, constructor or record header: directly in a class body, before any `=`.
        if matches!(top, Some(Frame::Brace(BraceKind::ClassBody)))
            && !(from..at).any(|i| self.is_punct(i, '='))
        {
            return ParenKind::Params;
        }
        ParenKind::Other
    }
}

/// Classifies the completion position at byte `offset` of `text`.
pub fn classify(text: &str, offset: usize) -> Classified {
    let offset = offset.min(text.len());
    let mut offset = offset;
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    let src = &text[..offset];
    let none = |prefix_start: usize| Classified {
        position: Position::None,
        prefix_start,
        prefix: text[prefix_start..offset].to_string(),
    };
    let (toks, end) = lex(src);
    if end != End::Code {
        return none(offset);
    }
    // The identifier prefix: a trailing identifier token adjacent to the caret.
    let mut n = toks.len();
    let mut prefix_start = offset;
    if let Some(last) = toks.last()
        && last.end == offset
    {
        match last.kind {
            Kind::Ident => {
                prefix_start = last.start;
                n -= 1;
            }
            Kind::Number | Kind::Literal => return none(offset),
            _ => {}
        }
    }
    let prefix = text[prefix_start..offset].to_string();
    let done = |position: Position| Classified {
        position,
        prefix_start,
        prefix: prefix.clone(),
    };
    let t = &toks[..n];
    let scan = Scan { src, toks: t };

    // Frame stack over the tokens before the prefix.
    let mut frames: Vec<(Frame, usize)> = Vec::new();
    let mut stmt = 0usize;
    #[allow(clippy::needless_range_loop)]
    for i in 0..t.len() {
        let top = frames.last().map(|(f, _)| *f);
        match t[i].kind {
            Kind::Punct('{') => {
                let kind = scan.brace_kind(stmt, i);
                frames.push((Frame::Brace(kind), stmt));
                stmt = i + 1;
            }
            Kind::Punct('}') => {
                while let Some((f, saved)) = frames.pop() {
                    if matches!(f, Frame::Brace(_)) {
                        let _ = saved;
                        break;
                    }
                }
                stmt = i + 1;
            }
            Kind::Punct('(') => {
                let kind = scan.paren_kind(stmt, i, top);
                frames.push((Frame::Paren(kind), stmt));
            }
            Kind::Punct(')') => {
                if let Some((Frame::Paren(_), saved)) = frames.last().copied() {
                    frames.pop();
                    let _ = saved;
                } else if let Some(pos) = frames
                    .iter()
                    .rposition(|(f, _)| matches!(f, Frame::Paren(_)))
                {
                    // An unbalanced brace inside the parenthesis (lambda body being typed).
                    frames.truncate(pos);
                }
            }
            Kind::Punct(';') if !matches!(top, Some(Frame::Paren(_))) => stmt = i + 1,
            _ => {}
        }
    }
    let top = frames.last().map(|(f, _)| *f);
    let header = &t[stmt.min(t.len())..];
    let prev = t.last();

    // No completion in import and package statements (the language server completes them).
    if header.first().is_some_and(|f| {
        f.kind == Kind::Ident && matches!(&src[f.start..f.end], "import" | "package")
    }) {
        return done(Position::None);
    }
    if matches!(top, Some(Frame::Paren(ParenKind::AnnotationArgs)))
        && !prev.is_some_and(|p| p.kind == Kind::Punct('@') && p.end == prefix_start)
    {
        return done(Position::None);
    }
    let Some(prev_tok) = prev.copied() else {
        return done(Position::None);
    };
    let pi = t.len() - 1;
    let prev_text = &src[prev_tok.start..prev_tok.end];

    // 1. `@|`
    if prev_tok.kind == Kind::Punct('@') && prev_tok.end == prefix_start {
        let after = &text[offset..];
        let target = annotation_target(after, top);
        return done(Position::Annotation { target });
    }
    // 2. qualified names: members and packages belong to the main contributor
    if prev_tok.kind == Kind::Punct('.') {
        return done(Position::None);
    }
    if prev_tok.kind == Kind::Ident {
        match prev_text {
            // 3. `new |`
            "new" => return done(Position::New),
            "instanceof" => return done(Position::Type(TypeFilter::Any)),
            "extends" | "implements" | "throws" | "permits" => {
                return done(Position::Type(header_filter(&scan, pi)));
            }
            _ => {}
        }
    }
    if matches!(prev_tok.kind, Kind::Punct(',') | Kind::Punct('&'))
        && let Some(filter) = list_filter(&scan, pi)
    {
        return done(Position::Type(filter));
    }
    // `catch (|`, `catch (A |`, `catch (final |`
    if matches!(top, Some(Frame::Paren(ParenKind::Catch)))
        && (prev_tok.kind == Kind::Punct('(')
            || prev_tok.kind == Kind::Punct('|')
            || scan.is_kw(pi, "final"))
    {
        return done(Position::Type(TypeFilter::Throwable));
    }
    // generic type arguments: `List<|`, `Map<String, |`, `? extends |`
    if prev_tok.kind == Kind::Punct('<') && pi >= 1 {
        let before = t[pi - 1];
        if before.kind == Kind::Ident {
            let name = &src[before.start..before.end];
            let decl_context = matches!(
                top,
                Some(Frame::Brace(BraceKind::ClassBody)) | Some(Frame::Paren(ParenKind::Params))
            );
            let new_before = pi >= 2 && scan.is_kw(pi - 2, "new");
            if decl_context || new_before || name.chars().next().is_some_and(char::is_uppercase) {
                return done(Position::Type(TypeFilter::Argument));
            }
        }
        // Otherwise a comparison: an expression position (below).
    }
    if prev_tok.kind == Kind::Ident
        && matches!(prev_text, "super" | "extends")
        && pi >= 1
        && t[pi - 1].kind == Kind::Punct('?')
    {
        return done(Position::Type(TypeFilter::Argument));
    }
    if prev_tok.kind == Kind::Punct(',') && angle_depth(&scan, pi) > 0 {
        return done(Position::Type(TypeFilter::Argument));
    }

    let is_modifier = prev_tok.kind == Kind::Ident && MODIFIERS.contains(&prev_text);
    match top {
        Some(Frame::Brace(BraceKind::ClassBody)) => {
            let annotation_end =
                prev_tok.kind == Kind::Ident && pi >= 1 && (t[pi - 1].kind == Kind::Punct('@'));
            let boundary = matches!(
                prev_tok.kind,
                Kind::Punct('{') | Kind::Punct('}') | Kind::Punct(';')
            ) || is_modifier
                || annotation_end
                || prev_tok.kind == Kind::Punct('>') && header_is_generic_start(&scan, pi);
            if boundary {
                return done(Position::Mixed { declaration: true });
            }
            if is_expression_start(&prev_tok, prev_text) {
                return done(Position::Mixed { declaration: false });
            }
            done(Position::None)
        }
        Some(Frame::Paren(ParenKind::Params)) => {
            if matches!(prev_tok.kind, Kind::Punct('(') | Kind::Punct(','))
                || prev_tok.kind == Kind::Ident && prev_text == "final"
            {
                return done(Position::Mixed { declaration: true });
            }
            done(Position::None)
        }
        Some(Frame::Paren(_)) | Some(Frame::Brace(BraceKind::Block | BraceKind::Other)) => {
            if matches!(
                prev_tok.kind,
                Kind::Punct('{') | Kind::Punct('}') | Kind::Punct(';')
            ) || is_expression_start(&prev_tok, prev_text)
            {
                return done(Position::Mixed {
                    declaration: is_modifier,
                });
            }
            if is_modifier {
                return done(Position::Mixed { declaration: true });
            }
            done(Position::None)
        }
        None => done(Position::None),
    }
}

/// A token after which an expression or a statement can start.
fn is_expression_start(tok: &Tok, text: &str) -> bool {
    match tok.kind {
        Kind::Arrow => true,
        Kind::Punct(c) => matches!(
            c,
            '(' | ','
                | '='
                | ':'
                | '?'
                | '+'
                | '-'
                | '*'
                | '/'
                | '%'
                | '!'
                | '<'
                | '>'
                | '&'
                | '|'
                | '^'
                | '~'
        ),
        Kind::Ident => matches!(
            text,
            "return" | "throw" | "else" | "case" | "do" | "assert" | "yield"
        ),
        _ => false,
    }
}

/// Whether the `>` at `i` ends the type parameters of a generic method or class header.
fn header_is_generic_start(scan: &Scan<'_>, i: usize) -> bool {
    let mut depth = 0i32;
    let mut j = i;
    loop {
        match scan.toks[j].kind {
            Kind::Punct('>') => depth += 1,
            Kind::Punct('<') => {
                depth -= 1;
                if depth == 0 {
                    // `<T> |`: the `<` starts a member declaration (after a modifier, a brace
                    // or a semicolon).
                    return j == 0
                        || matches!(
                            scan.toks[j - 1].kind,
                            Kind::Punct('{') | Kind::Punct('}') | Kind::Punct(';')
                        )
                        || scan.toks[j - 1].kind == Kind::Ident
                            && MODIFIERS.contains(&scan.text(j - 1));
                }
            }
            Kind::Ident
            | Kind::Punct(',')
            | Kind::Punct('?')
            | Kind::Punct('&')
            | Kind::Punct('.') => {}
            Kind::Punct('[') | Kind::Punct(']') => {}
            _ => return false,
        }
        if j == 0 {
            return false;
        }
        j -= 1;
    }
}

/// Number of unmatched `<` before token `i` in the current statement.
fn angle_depth(scan: &Scan<'_>, i: usize) -> i32 {
    let mut depth = 0i32;
    let mut j = i;
    while j > 0 {
        j -= 1;
        match scan.toks[j].kind {
            Kind::Punct('>') => depth -= 1,
            Kind::Punct('<') => depth += 1,
            Kind::Ident
            | Kind::Punct(',')
            | Kind::Punct('?')
            | Kind::Punct('.')
            | Kind::Punct('&')
            | Kind::Punct('[')
            | Kind::Punct(']') => {}
            _ => break,
        }
    }
    depth.max(0)
}

/// The filter of the keyword (`extends`, `implements`, `throws`, `permits`) at `i`.
fn header_filter(scan: &Scan<'_>, i: usize) -> TypeFilter {
    match scan.text(i) {
        "implements" => TypeFilter::Interface,
        "throws" => TypeFilter::Throwable,
        "permits" => TypeFilter::Any,
        _ => {
            // `extends`: a type-parameter bound, an interface's parents or a class's parent.
            if angle_depth(scan, i) > 0 {
                return TypeFilter::Any;
            }
            let mut j = i;
            while j > 0 {
                j -= 1;
                match scan.toks[j].kind {
                    Kind::Ident => match scan.text(j) {
                        "interface" => return TypeFilter::Interface,
                        "class" => return TypeFilter::ExtendsClass,
                        _ => {}
                    },
                    Kind::Punct(';') | Kind::Punct('{') | Kind::Punct('}') => break,
                    _ => {}
                }
            }
            TypeFilter::Any
        }
    }
}

/// `A, |` after `implements`/`throws`/`extends`/`permits`: the filter of the governing keyword.
fn list_filter(scan: &Scan<'_>, i: usize) -> Option<TypeFilter> {
    let mut depth = 0i32;
    let mut j = i;
    while j > 0 {
        j -= 1;
        match scan.toks[j].kind {
            Kind::Ident => match scan.text(j) {
                "extends" | "implements" | "throws" | "permits" => {
                    return Some(if depth > 0 {
                        TypeFilter::Argument
                    } else {
                        header_filter(scan, j)
                    });
                }
                _ => {}
            },
            Kind::Punct('>') => depth -= 1,
            Kind::Punct('<') => depth += 1,
            Kind::Punct(',')
            | Kind::Punct('.')
            | Kind::Punct('?')
            | Kind::Punct('&')
            | Kind::Punct('[')
            | Kind::Punct(']') => {}
            _ => return None,
        }
    }
    None
}

/// The element kind the annotation at the caret applies to, from the text after it:
/// modifiers and further annotations are skipped, then a class keyword (type), or a type and
/// a name followed by `(` (method), `=`/`;`/`,` (field, local variable, parameter).
fn annotation_target(after: &str, top: Option<Frame>) -> Option<u16> {
    // Skip the rest of the identifier being typed.
    let rest = after.trim_start_matches(is_ident_part);
    let (toks, _) = lex(&rest.chars().take(600).collect::<String>());
    let src: String = rest.chars().take(600).collect();
    let text = |i: usize| &src[toks[i].start..toks[i].end];
    let mut i = 0;
    // Annotations and modifiers in front of the declaration.
    loop {
        let t = toks.get(i)?;
        match t.kind {
            Kind::Punct('@') => {
                i += 1;
                if toks.get(i).is_some_and(|n| n.kind == Kind::Ident) && text(i) == "interface" {
                    return Some(target::TYPE | target::ANNOTATION_TYPE);
                }
                while toks.get(i).is_some_and(|n| n.kind == Kind::Ident)
                    || toks.get(i).is_some_and(|n| n.kind == Kind::Punct('.'))
                {
                    i += 1;
                }
                if toks.get(i).is_some_and(|n| n.kind == Kind::Punct('(')) {
                    let mut depth = 0;
                    while let Some(n) = toks.get(i) {
                        match n.kind {
                            Kind::Punct('(') => depth += 1,
                            Kind::Punct(')') => {
                                depth -= 1;
                                if depth == 0 {
                                    i += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                }
            }
            Kind::Ident if MODIFIERS.contains(&text(i)) => i += 1,
            _ => break,
        }
    }
    let t = toks.get(i)?;
    if t.kind != Kind::Ident {
        return None;
    }
    if matches!(text(i), "class" | "interface" | "enum" | "record") {
        return Some(target::TYPE);
    }
    // Generic method type parameters.
    if toks.get(i).is_some_and(|t| t.kind == Kind::Punct('<')) {
        let mut depth = 0;
        while let Some(n) = toks.get(i) {
            match n.kind {
                Kind::Punct('<') => depth += 1,
                Kind::Punct('>') => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
    // A type: name(.name)*, type arguments, array brackets.
    let first = toks.get(i)?;
    if first.kind != Kind::Ident {
        return None;
    }
    let constructor_candidate = toks.get(i + 1).is_some_and(|n| n.kind == Kind::Punct('('));
    if constructor_candidate {
        return Some(target::CONSTRUCTOR);
    }
    i += 1;
    loop {
        match toks.get(i).map(|t| t.kind) {
            Some(Kind::Punct('.')) => i += 2,
            Some(Kind::Punct('<')) => {
                let mut depth = 0;
                while let Some(n) = toks.get(i) {
                    match n.kind {
                        Kind::Punct('<') => depth += 1,
                        Kind::Punct('>') => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            Some(Kind::Punct('[')) => i += 2,
            _ => break,
        }
    }
    let name = toks.get(i)?;
    if name.kind != Kind::Ident {
        return None;
    }
    match toks.get(i + 1).map(|t| t.kind) {
        Some(Kind::Punct('(')) => Some(target::METHOD),
        Some(Kind::Punct('=')) | Some(Kind::Punct(';')) | Some(Kind::Punct(',')) => match top {
            Some(Frame::Brace(BraceKind::ClassBody)) => Some(target::FIELD),
            Some(Frame::Paren(ParenKind::Params)) => Some(target::PARAMETER),
            Some(Frame::Brace(_)) | Some(Frame::Paren(_)) => Some(target::LOCAL_VARIABLE),
            None => None,
        },
        Some(Kind::Punct(')')) => match top {
            Some(Frame::Paren(ParenKind::Params)) => Some(target::PARAMETER),
            _ => None,
        },
        _ => None,
    }
}

/// Whether an annotation type with `targets` (`None`: no `@Target`) may annotate `wanted`.
pub fn target_fits(targets: Option<u16>, wanted: Option<u16>) -> bool {
    let (Some(mask), Some(wanted)) = (targets, wanted) else {
        return true;
    };
    let wanted_mask = match wanted {
        target::METHOD => target::METHOD | target::TYPE_USE,
        target::FIELD => target::FIELD | target::TYPE_USE,
        target::PARAMETER => target::PARAMETER | target::TYPE_USE,
        target::LOCAL_VARIABLE => target::LOCAL_VARIABLE | target::TYPE_USE,
        target::CONSTRUCTOR => target::CONSTRUCTOR | target::TYPE_USE,
        w => w | target::TYPE_USE,
    };
    mask & wanted_mask != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(src: &str) -> Classified {
        let offset = src.find('|').expect("caret");
        let text = src.replacen('|', "", 1);
        classify(&text, offset)
    }

    #[test]
    fn annotation_before_a_method() {
        let c = at("class A {\n  @Te|\n  void f() {}\n}");
        assert_eq!(c.prefix, "Te");
        assert_eq!(
            c.position,
            Position::Annotation {
                target: Some(target::METHOD)
            }
        );
    }

    #[test]
    fn annotation_targets() {
        let c = at("class A {\n  @|\n  private int x;\n}");
        assert_eq!(
            c.position,
            Position::Annotation {
                target: Some(target::FIELD)
            }
        );
        let c = at("@|\npublic class A {}");
        assert_eq!(
            c.position,
            Position::Annotation {
                target: Some(target::TYPE)
            }
        );
        let c = at("class A { void f(@| String s) {} }");
        assert_eq!(
            c.position,
            Position::Annotation {
                target: Some(target::PARAMETER)
            }
        );
        let c = at("class A { void f() { @| var x = 1; } }");
        assert!(matches!(c.position, Position::Annotation { .. }));
    }

    #[test]
    fn after_new() {
        let c = at("class A { void f() { Object o = new ArrayLi|; } }");
        assert_eq!(c.position, Position::New);
        assert_eq!(c.prefix, "ArrayLi");
        let c = at("class A { void f() { Object o = new |; } }");
        assert_eq!(c.position, Position::New);
        assert_eq!(c.prefix, "");
    }

    #[test]
    fn header_positions() {
        assert_eq!(
            at("class A extends Ba| {}").position,
            Position::Type(TypeFilter::ExtendsClass)
        );
        assert_eq!(
            at("interface A extends Ba| {}").position,
            Position::Type(TypeFilter::Interface)
        );
        assert_eq!(
            at("class A implements Ru| {}").position,
            Position::Type(TypeFilter::Interface)
        );
        assert_eq!(
            at("class A implements Runnable, Co| {}").position,
            Position::Type(TypeFilter::Interface)
        );
        assert_eq!(
            at("class A { void f() throws Ex| {} }").position,
            Position::Type(TypeFilter::Throwable)
        );
        assert_eq!(
            at("class A { void f() { try { } catch (Ex| e) { } } }").position,
            Position::Type(TypeFilter::Throwable)
        );
        assert_eq!(
            at("class A { void f(Object o) { if (o instanceof Str|) { } } }").position,
            Position::Type(TypeFilter::Any)
        );
    }

    #[test]
    fn generics() {
        assert_eq!(
            at("class A { List<Str|> xs; }").position,
            Position::Type(TypeFilter::Argument)
        );
        assert_eq!(
            at("class A { Map<String, Li|> xs; }").position,
            Position::Type(TypeFilter::Argument)
        );
        assert_eq!(
            at("class A { void f() { int i = 0; boolean b = i < Ma|; } }").position,
            Position::Mixed { declaration: false }
        );
    }

    #[test]
    fn declarations_and_statements() {
        assert_eq!(
            at("class A { Str| }").position,
            Position::Mixed { declaration: true }
        );
        assert_eq!(
            at("class A { private Li| }").position,
            Position::Mixed { declaration: true }
        );
        assert_eq!(
            at("class A { void f(Str| s) {} }").position,
            Position::Mixed { declaration: true }
        );
        assert_eq!(
            at("class A { void f() { Str| } }").position,
            Position::Mixed { declaration: false }
        );
        assert_eq!(
            at("class A { void f() { int x = Ma|; } }").position,
            Position::Mixed { declaration: false }
        );
    }

    #[test]
    fn nothing_to_complete() {
        assert_eq!(
            at("class A { void f() { int x = 1; x.fo| } }").position,
            Position::None
        );
        assert_eq!(at("class A { // Te|\n }").position, Position::None);
        assert_eq!(at("class A { /* @Te| */ }").position, Position::None);
        assert_eq!(
            at("class A { String s = \"@Te|\"; }").position,
            Position::None
        );
        assert_eq!(at("class A { /** @Te| */ }").position, Position::None);
        assert_eq!(at("import java.util.Li|;").position, Position::None);
        assert_eq!(at("class A { int x = 5|; }").position, Position::None);
        assert_eq!(at("class A { int co| }").position, Position::None);
        assert_eq!(
            at("class A { @Foo(na|) void f() {} }").position,
            Position::None
        );
    }

    #[test]
    fn braces_of_methods_and_lambdas_are_blocks() {
        assert_eq!(
            at("class A { void f() { Runnable r = () -> { Str| }; } }").position,
            Position::Mixed { declaration: false }
        );
        assert_eq!(
            at("class A { void f() { new Thread() { void run() { } Str| }; } }").position,
            Position::Mixed { declaration: true }
        );
    }
}
