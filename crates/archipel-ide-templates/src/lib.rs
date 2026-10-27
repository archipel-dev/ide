//! Live templates and postfix templates (IJ-ED-130/131/086).
//!
//! Templates are data: bundled JSON sets per language (`templates/*.json`) with
//! `$NAME$` variables, `$END$` (final caret) and `$SELECTION$` (surround
//! templates). An expanded [`Instance`] converts to LSP snippet syntax, so the
//! editor's snippet session handles Tab stops and mirrored variables.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

mod context;

pub use context::{Context, Language, Scan, contexts_at, scan};

/// A variable's definition inside a template.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VariableDef {
    pub name: String,
    /// Expression computing the initial value, e.g. `className()`,
    /// `suggestIndexName()`, `"literal"`.
    pub expression: Option<String>,
    /// Value used when the expression yields nothing.
    pub default: Option<String>,
    /// Do not stop on this variable when the expression produced a value.
    pub skip_if_defined: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Template {
    pub abbreviation: String,
    pub description: String,
    /// Body with `$VAR$`, `$END$`, `$SELECTION$`; `$$` is a literal `$`.
    /// Nested lines are written with 4-space indentation steps.
    pub text: String,
    pub variables: Vec<VariableDef>,
    /// Contexts where the template applies; empty means every context of its set.
    pub contexts: Vec<Context>,
}

impl Template {
    pub fn applies_in(&self, contexts: &[Context]) -> bool {
        self.contexts.is_empty() || self.contexts.iter().any(|c| contexts.contains(c))
    }

    /// Surround templates wrap the selection (`$SELECTION$`).
    pub fn is_surround(&self) -> bool {
        self.text.contains("$SELECTION$")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateSet {
    /// Group name shown in settings, e.g. `Java`.
    pub group: String,
    /// File extensions the set applies to; empty means every file.
    pub extensions: Vec<String>,
    pub templates: Vec<Template>,
    /// Postfix templates: `abbreviation` is the key after the dot and `$EXPR$`
    /// is the expression before it.
    pub postfix: Vec<Template>,
}

impl TemplateSet {
    fn applies_to(&self, path: &Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase);
        extension.is_some_and(|ext| self.extensions.contains(&ext))
    }
}

const BUNDLED: [(&str, &str); 4] = [
    ("java.json", include_str!("../templates/java.json")),
    ("kotlin.json", include_str!("../templates/kotlin.json")),
    ("rust.json", include_str!("../templates/rust.json")),
    ("other.json", include_str!("../templates/other.json")),
];

/// A template the user created or edited, stored as one JSON line in settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTemplate {
    /// Group name; joins the bundled set of that name (and its file types) or
    /// creates a set that applies to every file.
    pub group: String,
    #[serde(default)]
    pub postfix: bool,
    pub template: Template,
}

impl UserTemplate {
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_line(line: &str) -> Result<Self, String> {
        let user: Self = serde_json::from_str(line).map_err(|error| error.to_string())?;
        if user.template.abbreviation.trim().is_empty() {
            return Err("a live template needs an abbreviation".into());
        }
        Ok(user)
    }
}

/// Settings key of a template, used for the enable toggles. Postfix
/// templates are keyed `group/.abbreviation`, surround ones `group/abbreviation`.
pub fn template_key(group: &str, template: &Template, postfix: bool) -> String {
    format!(
        "{group}/{}{}",
        if postfix { "." } else { "" },
        template.abbreviation
    )
}

#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    pub sets: Vec<TemplateSet>,
    /// Bundled or user sets that failed to parse, with the reason.
    pub errors: Vec<String>,
    /// Keys ([`template_key`]) or group names switched off in settings.
    pub disabled: HashSet<String>,
}

impl TemplateRegistry {
    /// Parses sets; malformed sets are reported in `errors` and skipped.
    pub fn from_sources<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        let mut registry = Self::default();
        for (name, json) in sources {
            match serde_json::from_str::<TemplateSet>(json) {
                Ok(set) => registry.sets.push(set),
                Err(error) => registry.errors.push(format!("{name}: {error}")),
            }
        }
        registry
    }

    /// The bundled Java, Kotlin, Rust and language-independent sets.
    pub fn bundled() -> &'static Self {
        static REGISTRY: OnceLock<TemplateRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| Self::from_sources(BUNDLED))
    }

    /// This registry plus the user's templates (which win over bundled ones of
    /// the same abbreviation) and the switched-off keys. Unparseable user
    /// lines are reported in `errors`.
    pub fn with_user(
        &self,
        lines: impl IntoIterator<Item = impl AsRef<str>>,
        disabled: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut registry = self.clone();
        registry.disabled = disabled.into_iter().collect();
        for line in lines {
            let user = match UserTemplate::from_line(line.as_ref()) {
                Ok(user) => user,
                Err(error) => {
                    registry.errors.push(format!("user template: {error}"));
                    continue;
                }
            };
            let set = match registry
                .sets
                .iter_mut()
                .find(|set| set.group.eq_ignore_ascii_case(&user.group))
            {
                Some(set) => set,
                None => {
                    registry.sets.push(TemplateSet {
                        group: user.group.clone(),
                        ..Default::default()
                    });
                    registry.sets.last_mut().expect("just pushed")
                }
            };
            let list = if user.postfix {
                &mut set.postfix
            } else {
                &mut set.templates
            };
            list.retain(|t| {
                !(t.abbreviation == user.template.abbreviation
                    && t.is_surround() == user.template.is_surround())
            });
            list.insert(0, user.template);
        }
        registry
    }

    pub fn is_enabled(&self, set: &TemplateSet, template: &Template, postfix: bool) -> bool {
        !self.disabled.contains(&set.group)
            && !self
                .disabled
                .contains(&template_key(&set.group, template, postfix))
    }

    fn ordered_sets(&self, path: &Path) -> Vec<&TemplateSet> {
        let (specific, general): (Vec<&TemplateSet>, Vec<&TemplateSet>) = self
            .sets
            .iter()
            .filter(|set| set.applies_to(path))
            .partition(|set| !set.extensions.is_empty());
        specific.into_iter().chain(general).collect()
    }

    /// Enabled expandable (non-surround) templates applicable to `path`, language sets first.
    pub fn templates_for<'a>(&'a self, path: &Path) -> impl Iterator<Item = &'a Template> + 'a {
        self.ordered_sets(path).into_iter().flat_map(move |set| {
            set.templates
                .iter()
                .filter(move |t| !t.is_surround() && self.is_enabled(set, t, false))
        })
    }

    /// Expandable templates whose context holds.
    pub fn templates_in<'a>(
        &'a self,
        path: &Path,
        contexts: &'a [Context],
    ) -> impl Iterator<Item = &'a Template> + 'a {
        self.templates_for(path).filter(|t| t.applies_in(contexts))
    }

    /// Enabled surround templates (`$SELECTION$`) whose context holds.
    pub fn surround_templates<'a>(
        &'a self,
        path: &Path,
        contexts: &'a [Context],
    ) -> impl Iterator<Item = &'a Template> + 'a {
        self.ordered_sets(path).into_iter().flat_map(move |set| {
            set.templates.iter().filter(move |t| {
                t.is_surround() && self.is_enabled(set, t, false) && t.applies_in(contexts)
            })
        })
    }

    pub fn find(&self, path: &Path, abbreviation: &str) -> Option<&Template> {
        self.templates_for(path)
            .find(|template| template.abbreviation == abbreviation)
    }

    /// The template `abbreviation` expands to at the caret, honoring contexts.
    pub fn find_in(
        &self,
        path: &Path,
        contexts: &[Context],
        abbreviation: &str,
    ) -> Option<&Template> {
        self.templates_for(path)
            .find(|t| t.abbreviation == abbreviation && t.applies_in(contexts))
    }

    /// Enabled postfix templates for `path`.
    pub fn postfix_for<'a>(&'a self, path: &Path) -> impl Iterator<Item = &'a Template> + 'a {
        self.ordered_sets(path).into_iter().flat_map(move |set| {
            set.postfix
                .iter()
                .filter(move |t| self.is_enabled(set, t, true))
        })
    }

    pub fn find_postfix(&self, path: &Path, key: &str) -> Option<&Template> {
        self.postfix_for(path)
            .find(|template| template.abbreviation == key)
    }
}

/// Inputs for computing variable values and indentation.
#[derive(Debug, Clone, Default)]
pub struct ExpansionContext {
    /// File name, e.g. `Main.java`.
    pub file_name: String,
    /// Indentation of the line the template is inserted on.
    pub line_indent: String,
    /// One indentation level (`"    "` or `"\t"`).
    pub indent_unit: String,
    /// Selected text for surround templates.
    pub selection: Option<String>,
    /// Expression for postfix templates (`$EXPR$`).
    pub expression: Option<String>,
    /// The word before the abbreviation a template with `$ARG$` consumes (ED-TMPL-012).
    pub argument: Option<String>,
    /// Nearby text, used to pick unused names (`suggestIndexName()`).
    pub surrounding: String,
    /// Enclosing type and method (`className()`, `methodName()`, `methodParameters()`).
    pub class_name: Option<String>,
    pub method_name: Option<String>,
    pub method_parameters: Vec<String>,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Evaluates a variable expression; `None` when it yields nothing.
pub fn evaluate(expression: &str, context: &ExpansionContext) -> Option<String> {
    let expression = expression.trim();
    if let Some(literal) = expression
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return Some(literal.to_owned());
    }
    let stem = || {
        Path::new(&context.file_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_owned)
    };
    match expression {
        "className()" => context.class_name.clone().or_else(stem),
        "fileNameWithoutExtension()" => stem(),
        "fileName()" => Some(context.file_name.clone()),
        "methodName()" => context.method_name.clone(),
        "methodParameters()" => {
            let params: Vec<String> = context
                .method_parameters
                .iter()
                .map(|p| format!("\"{p} = \" + {p}"))
                .collect();
            (!params.is_empty()).then(|| params.join(" + \", \" + "))
        }
        "expression()" => context.expression.clone(),
        "suggestVariableName()" => context.expression.as_deref().map(suggest_variable_name),
        "expressionType()" => context.expression.as_deref().and_then(expression_type),
        "typeOrVar()" => Some(
            context
                .expression
                .as_deref()
                .and_then(expression_type)
                .unwrap_or_else(|| "var".into()),
        ),
        "suggestIndexName()" => {
            let used = |name: &str| {
                context
                    .surrounding
                    .split(|c: char| !is_word(c))
                    .any(|word| word == name)
            };
            ["i", "j", "k", "l", "m", "n"]
                .into_iter()
                .find(|name| !used(name))
                .map(str::to_owned)
        }
        _ => None,
    }
}

/// A variable name for the value of `expression`: `getUserName()` -> `userName`,
/// `new Foo()` -> `foo`, `items.size()` -> `size`, literals -> `value`.
pub fn suggest_variable_name(expression: &str) -> String {
    let expression = expression.trim();
    let source = expression.strip_prefix("new ").unwrap_or(expression);
    // Last identifier outside of any trailing call/index brackets.
    let mut head = source;
    loop {
        let trimmed = head.trim_end();
        let Some(last) = trimmed.chars().last() else {
            break;
        };
        if !matches!(last, ')' | ']') {
            head = trimmed;
            break;
        }
        let (open, close) = if last == ')' { ('(', ')') } else { ('[', ']') };
        let mut depth = 0usize;
        let mut cut = None;
        for (index, c) in trimmed.char_indices().rev() {
            if c == close {
                depth += 1;
            } else if c == open {
                depth -= 1;
                if depth == 0 {
                    cut = Some(index);
                    break;
                }
            }
        }
        match cut {
            Some(index) => head = &trimmed[..index],
            None => break,
        }
    }
    let head = match (head.trim_end().ends_with('>'), head.find('<')) {
        (true, Some(open)) => &head[..open],
        _ => head,
    };
    let name: String = head
        .chars()
        .rev()
        .take_while(|c| is_word(*c))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let stripped = ["get", "to", "as", "is", "has"]
        .into_iter()
        .find_map(|prefix| {
            name.strip_prefix(prefix)
                .filter(|rest| rest.chars().next().is_some_and(char::is_uppercase))
        })
        .unwrap_or(&name);
    let mut chars = stripped.chars();
    match chars.next() {
        Some(first) if first.is_alphabetic() => {
            let rest: String = chars.collect();
            // Keep acronyms (`URL`) intact.
            if rest.chars().next().is_some_and(char::is_uppercase) {
                stripped.to_owned()
            } else {
                format!("{}{rest}", first.to_lowercase())
            }
        }
        _ => "value".to_owned(),
    }
}

/// Best-effort type of a literal or constructor expression.
pub fn expression_type(expression: &str) -> Option<String> {
    let expression = expression.trim();
    if expression.starts_with('"') {
        return Some("String".into());
    }
    if expression == "true" || expression == "false" {
        return Some("boolean".into());
    }
    if expression.starts_with('\'') {
        return Some("char".into());
    }
    if expression
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_digit())
    {
        let float = expression.contains('.') || expression.ends_with(['f', 'F', 'd', 'D']);
        return Some(
            match (float, expression.ends_with(['l', 'L'])) {
                (true, _) => "double",
                (false, true) => "long",
                _ => "int",
            }
            .into(),
        );
    }
    let class = expression.strip_prefix("new ")?;
    let end = class.find(['(', '[', '{']).unwrap_or(class.len());
    let class = class[..end].trim();
    (!class.is_empty()).then(|| class.replace("<>", ""))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Segment {
    Text(String),
    Var(usize),
    End,
}

/// A template with resolved text segments and initial variable values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub(crate) segments: Vec<Segment>,
    pub(crate) names: Vec<String>,
    pub(crate) values: Vec<String>,
    /// Variables visited by Tab, in order of first appearance.
    pub(crate) stops: Vec<usize>,
}

impl Instance {
    pub fn text(&self) -> String {
        self.segments
            .iter()
            .map(|segment| match segment {
                Segment::Text(text) => text.as_str(),
                Segment::Var(index) => self.values[*index].as_str(),
                Segment::End => "",
            })
            .collect()
    }

    pub fn variable_names(&self) -> &[String] {
        &self.names
    }

    /// The expansion in LSP snippet syntax: variables that stop the caret
    /// become numbered placeholders (mirrors share the number), `$END$`
    /// becomes `$0`, everything else is escaped text.
    pub fn to_snippet(&self) -> String {
        fn escape(text: &str, in_placeholder: bool, out: &mut String) {
            for c in text.chars() {
                if matches!(c, '\\' | '$') || (in_placeholder && c == '}') {
                    out.push('\\');
                }
                out.push(c);
            }
        }
        let mut out = String::new();
        let mut seen = HashSet::new();
        for segment in &self.segments {
            match segment {
                Segment::Text(text) => escape(text, false, &mut out),
                Segment::End => out.push_str("$0"),
                Segment::Var(index) => match self.stops.iter().position(|s| s == index) {
                    None => escape(&self.values[*index], false, &mut out),
                    Some(stop) if seen.insert(*index) => {
                        out.push_str(&format!("${{{}:", stop + 1));
                        escape(&self.values[*index], true, &mut out);
                        out.push('}');
                    }
                    Some(stop) => out.push_str(&format!("${{{}}}", stop + 1)),
                },
            }
        }
        out
    }
}

/// Reindents template line starts: each leading 4-space step becomes one
/// indent unit, and every line after the first gets the insertion line's indent.
fn indent_text(text: &str, at_line_start: bool, context: &ExpansionContext) -> String {
    let mut out = String::new();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        if index > 0 || at_line_start {
            let spaces = line.len() - line.trim_start_matches(' ').len();
            let (steps, rest) = (spaces / 4, &line[spaces - spaces % 4..]);
            if index > 0 && !line.is_empty() {
                out.push_str(&context.line_indent);
            }
            for _ in 0..steps {
                out.push_str(&context.indent_unit);
            }
            out.push_str(rest);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Selection text re-indented to sit at `indent` inside the template.
fn reindent_selection(selection: &str, indent: &str) -> String {
    let lines: Vec<&str> = selection.trim_end_matches('\n').split('\n').collect();
    let common = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start_matches([' ', '\t']).len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let body = line.get(common..).unwrap_or(line.trim_start());
            if index == 0 || body.is_empty() {
                body.to_owned()
            } else {
                format!("{indent}{body}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parses `template` into segments and computes initial variable values.
pub fn instantiate(template: &Template, context: &ExpansionContext) -> Instance {
    let mut segments = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut literal = String::new();
    let mut rest = template.text.as_str();
    let mut at_line_start = false;
    let flush = |literal: &mut String, segments: &mut Vec<Segment>, at_line_start: &mut bool| {
        if !literal.is_empty() {
            let text = indent_text(literal, *at_line_start, context);
            *at_line_start = literal.ends_with('\n');
            segments.push(Segment::Text(text));
            literal.clear();
        }
    };
    while let Some(dollar) = rest.find('$') {
        literal.push_str(&rest[..dollar]);
        let after = &rest[dollar + 1..];
        if let Some(stripped) = after.strip_prefix('$') {
            literal.push('$');
            rest = stripped;
            continue;
        }
        let name_len = after
            .find(|c: char| !is_word(c))
            .filter(|&end| end > 0 && after[end..].starts_with('$'));
        let Some(name_len) = name_len else {
            // A lone `$` is literal text.
            literal.push('$');
            rest = after;
            continue;
        };
        let name = &after[..name_len];
        rest = &after[name_len + 1..];
        // Indentation of the current template line, for `$SELECTION$`.
        let line_prefix = literal.rsplit('\n').next().unwrap_or("").to_owned();
        let on_new_line = literal.contains('\n') || at_line_start;
        flush(&mut literal, &mut segments, &mut at_line_start);
        match name {
            "END" => segments.push(Segment::End),
            "SELECTION" => {
                let steps = line_prefix.len() / 4;
                let mut indent = if on_new_line {
                    context.line_indent.clone()
                } else {
                    String::new()
                };
                for _ in 0..steps {
                    indent.push_str(&context.indent_unit);
                }
                let text = reindent_selection(context.selection.as_deref().unwrap_or(""), &indent);
                segments.push(Segment::Text(text));
            }
            _ => {
                let index = names.iter().position(|n| n == name).unwrap_or_else(|| {
                    names.push(name.to_owned());
                    names.len() - 1
                });
                segments.push(Segment::Var(index));
            }
        }
    }
    literal.push_str(rest);
    flush(&mut literal, &mut segments, &mut at_line_start);

    let mut values = Vec::with_capacity(names.len());
    let mut stops = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let definition = template.variables.iter().find(|v| &v.name == name);
        let computed = if name == "EXPR" {
            context.expression.clone()
        } else if name == "ARG" {
            context.argument.clone()
        } else {
            definition
                .and_then(|d| d.expression.as_deref())
                .and_then(|expression| evaluate(expression, context))
        };
        let skip = name == "EXPR"
            || name == "ARG"
            || (computed.is_some() && definition.is_some_and(|d| d.skip_if_defined));
        let value = computed
            .or_else(|| definition.and_then(|d| d.default.clone()))
            .unwrap_or_default();
        values.push(value);
        if !skip {
            stops.push(index);
        }
    }
    Instance {
        segments,
        names,
        values,
        stops,
    }
}

/// Characters allowed in abbreviations (`else-if`, `psvm`).
fn is_abbreviation_char(c: char) -> bool {
    is_word(c) || c == '-'
}

/// The abbreviation ending at `offset` and its start, if any.
pub fn abbreviation_before(
    doc: &archipel_ide_text::TextDocument,
    offset: usize,
) -> Option<(usize, String)> {
    let mut start = offset;
    while start > 0 && doc.char_at(start - 1).is_some_and(is_abbreviation_char) {
        start -= 1;
    }
    // `a.sout` is postfix syntax, not the `sout` live template.
    let after_dot = start > 0 && doc.char_at(start - 1) == Some('.');
    (start < offset && !after_dot)
        .then(|| doc.slice(start, offset).ok())
        .flatten()
        .map(|text| (start, text))
}

/// Start of the expression that ends at `dot` (the `.` of `expr.key`). Scans
/// back over identifiers, member access, calls/indexing with balanced brackets
/// and string literals, stopping at whitespace or operators at depth zero.
pub fn expression_before(doc: &archipel_ide_text::TextDocument, dot: usize) -> Option<usize> {
    const LIMIT: usize = 2_000;
    let mut index = dot;
    let mut depth = 0usize;
    let mut steps = 0;
    while index > 0 && steps < LIMIT {
        steps += 1;
        let c = doc.char_at(index - 1)?;
        match c {
            ')' | ']' => depth += 1,
            '(' | '[' if depth > 0 => depth -= 1,
            '(' | '[' => break,
            '"' | '\'' => {
                // Skip a string literal backwards.
                let quote = c;
                index -= 1;
                while index > 0 && doc.char_at(index - 1) != Some(quote) {
                    index -= 1;
                }
                if index == 0 {
                    return None;
                }
            }
            _ if depth > 0 => {}
            // Generic arguments directly after a type name: `new Foo<A, B>()`.
            '>' => {
                let mut angle = 0usize;
                let mut open = None;
                for back in (index.saturating_sub(200)..index - 1).rev() {
                    match doc.char_at(back)? {
                        '>' => angle += 1,
                        '<' if angle == 0 => {
                            open = Some(back);
                            break;
                        }
                        '<' => angle -= 1,
                        c if is_word(c) || matches!(c, ',' | '.' | '?' | ' ' | '[' | ']') => {}
                        _ => break,
                    }
                }
                match open.filter(|&open| open > 0 && doc.char_at(open - 1).is_some_and(is_word)) {
                    Some(open) => index = open + 1,
                    None => break,
                }
            }
            c if is_word(c) || c == '.' || c == ':' || c == '!' && index == dot => {}
            _ => break,
        }
        index -= 1;
    }
    if index >= 4
        && doc.slice(index - 4, index).is_ok_and(|text| text == "new ")
        && (index == 4 || doc.char_at(index - 5).is_some_and(|c| !is_word(c)))
    {
        index -= 4;
    }
    (index < dot && depth == 0).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_text::TextDocument;

    fn context() -> ExpansionContext {
        ExpansionContext {
            file_name: "Main.java".into(),
            line_indent: "    ".into(),
            indent_unit: "    ".into(),
            ..Default::default()
        }
    }

    #[test]
    fn bundled_sets_parse_and_have_unique_abbreviations() {
        let registry = TemplateRegistry::bundled();
        assert!(registry.errors.is_empty(), "{:?}", registry.errors);
        assert_eq!(registry.sets.len(), BUNDLED.len());
        for set in &registry.sets {
            let mut seen = std::collections::HashSet::new();
            for template in set.templates.iter().chain(&set.postfix) {
                assert!(
                    seen.insert((
                        template.is_surround(),
                        set.postfix.contains(template),
                        &template.abbreviation
                    )),
                    "{}: duplicate {}",
                    set.group,
                    template.abbreviation
                );
                assert!(!template.description.is_empty());
            }
        }
        let java = Path::new("A.java");
        for abbreviation in [
            "psvm", "main", "sout", "soutv", "fori", "iter", "ifn", "inn", "thr",
        ] {
            assert!(
                registry.find(java, abbreviation).is_some(),
                "{abbreviation}"
            );
        }
        assert!(registry.find(Path::new("a.kt"), "fun0").is_some());
        assert!(registry.find(Path::new("a.rs"), "test").is_some());
        assert!(
            registry.find(java, "todo").is_some(),
            "general set applies everywhere"
        );
        assert!(registry.find(Path::new("a.rs"), "psvm").is_none());
        assert!(registry.find_postfix(java, "if").is_some());
    }

    #[test]
    fn malformed_sets_are_reported_not_fatal() {
        let registry = TemplateRegistry::from_sources([("bad.json", "{"), ("ok.json", "{}")]);
        assert_eq!(registry.sets.len(), 1);
        assert!(registry.errors[0].starts_with("bad.json"));
    }

    #[test]
    fn instantiates_variables_end_and_indentation() {
        let template = Template {
            abbreviation: "fori".into(),
            text: "for (int $INDEX$ = 0; $INDEX$ < $LIMIT$; $INDEX$++) {\n    $END$\n}".into(),
            variables: vec![VariableDef {
                name: "INDEX".into(),
                expression: Some("suggestIndexName()".into()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut ctx = context();
        ctx.surrounding = "int i = 0;".into();
        let instance = instantiate(&template, &ctx);
        assert_eq!(
            instance.text(),
            "for (int j = 0; j < ; j++) {\n        \n    }"
        );
        assert_eq!(instance.variable_names(), ["INDEX", "LIMIT"]);
        assert_eq!(instance.stops, [0, 1]);
    }

    #[test]
    fn evaluates_expressions_and_skips_defined_variables() {
        let ctx = context();
        assert_eq!(evaluate("className()", &ctx).as_deref(), Some("Main"));
        assert_eq!(evaluate("\"x\"", &ctx).as_deref(), Some("x"));
        assert_eq!(evaluate("suggestIndexName()", &ctx).as_deref(), Some("i"));
        assert_eq!(evaluate("unknown()", &ctx), None);
        let template = Template {
            text: "class $NAME$ $BODY$".into(),
            variables: vec![
                VariableDef {
                    name: "NAME".into(),
                    expression: Some("className()".into()),
                    skip_if_defined: true,
                    ..Default::default()
                },
                VariableDef {
                    name: "BODY".into(),
                    default: Some("{}".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let instance = instantiate(&template, &ctx);
        assert_eq!(instance.text(), "class Main {}");
        assert_eq!(instance.stops, [1]);
    }

    #[test]
    fn surround_reindents_the_selection() {
        let template = Template {
            text: "if ($COND$) {\n    $SELECTION$\n}".into(),
            ..Default::default()
        };
        let mut ctx = context();
        ctx.selection = Some("    a();\n        b();\n".into());
        assert_eq!(
            instantiate(&template, &ctx).text(),
            "if () {\n        a();\n            b();\n    }"
        );
    }

    #[test]
    fn literal_dollars_and_unclosed_variables_stay_text() {
        let template = Template {
            text: "$$x $5 $A".into(),
            ..Default::default()
        };
        assert_eq!(instantiate(&template, &context()).text(), "$x $5 $A");
    }

    #[test]
    fn finds_abbreviations_and_postfix_expressions() {
        let doc = TextDocument::new("  sout x.sout foo(a, b)[0].size().if \"s\".len");
        assert_eq!(abbreviation_before(&doc, 6), Some((2, "sout".into())));
        assert_eq!(
            abbreviation_before(&doc, 13),
            None,
            "postfix, not a template"
        );
        assert_eq!(abbreviation_before(&doc, 2), None);
        let dot = 33;
        assert_eq!(doc.char_at(dot), Some('.'));
        let start = expression_before(&doc, dot).unwrap();
        assert_eq!(doc.slice(start, dot).unwrap(), "foo(a, b)[0].size()");
        let dot = 40;
        assert_eq!(doc.char_at(dot), Some('.'));
        assert_eq!(
            doc.slice(expression_before(&doc, dot).unwrap(), dot)
                .unwrap(),
            "\"s\""
        );
        assert_eq!(expression_before(&TextDocument::new(" .x"), 1), None);
        let generic = TextDocument::new("x = new Map<String, List<Integer>>().size");
        let dot = generic.text().rfind('.').unwrap();
        let start = expression_before(&generic, dot).unwrap();
        assert_eq!(
            generic.slice(start, dot).unwrap(),
            "new Map<String, List<Integer>>()"
        );
        let compare = TextDocument::new("a > b.if");
        assert_eq!(
            compare
                .slice(expression_before(&compare, 5).unwrap(), 5)
                .unwrap(),
            "b"
        );
    }

    #[test]
    fn snippet_conversion_numbers_stops_and_mirrors() {
        let registry = TemplateRegistry::bundled();
        let fori = registry.find(Path::new("A.java"), "fori").unwrap();
        let instance = instantiate(
            fori,
            &ExpansionContext {
                indent_unit: "    ".into(),
                ..Default::default()
            },
        );
        assert_eq!(
            instance.to_snippet(),
            "for (int ${1:i} = 0; ${1} < ${2:}; ${1}++) {\n    $0\n}"
        );
        let ctx = ExpansionContext {
            surrounding: "int i;".into(),
            ..Default::default()
        };
        let sout = instantiate(registry.find(Path::new("A.java"), "soutv").unwrap(), &ctx);
        assert_eq!(
            sout.to_snippet(),
            "System.out.println(\"${1:} = \" + ${1});"
        );
        let literal = Template {
            text: "a${b}\\$$".into(),
            ..Default::default()
        };
        assert_eq!(
            instantiate(&literal, &ExpansionContext::default()).to_snippet(),
            "a\\${b}\\\\\\$"
        );
    }

    #[test]
    fn postfix_templates_rewrite_the_expression() {
        let registry = TemplateRegistry::bundled();
        let java = Path::new("A.java");
        let ctx = |expression: &str| ExpansionContext {
            expression: Some(expression.into()),
            indent_unit: "    ".into(),
            ..Default::default()
        };
        let text = |key: &str, expr: &str| {
            instantiate(registry.find_postfix(java, key).unwrap(), &ctx(expr)).text()
        };
        assert_eq!(text("var", "new Foo()"), "Foo foo = new Foo();");
        assert_eq!(
            text("var", "getUserName()"),
            "var userName = getUserName();"
        );
        assert_eq!(text("not", "ready"), "!ready");
        assert_eq!(text("sout", "x + 1"), "System.out.println(x + 1);");
        assert_eq!(text("par", "a"), "(a)");
        assert_eq!(text("nn", "p"), "if (p != null) {\n    \n}");
        assert_eq!(
            instantiate(
                registry.find_postfix(Path::new("a.kt"), "val").unwrap(),
                &ctx("items.size()")
            )
            .text(),
            "val size = items.size()"
        );
        assert!(registry.find_postfix(Path::new("a.rs"), "if").is_none());
    }

    #[test]
    fn suggests_names_and_types() {
        assert_eq!(suggest_variable_name("getURL()"), "URL");
        assert_eq!(suggest_variable_name("a.b.isReady()"), "ready");
        assert_eq!(suggest_variable_name("new ArrayList<>()"), "arrayList");
        assert_eq!(suggest_variable_name("list.get(0)"), "get");
        assert_eq!(suggest_variable_name("\"text\""), "value");
        assert_eq!(expression_type("1L").as_deref(), Some("long"));
        assert_eq!(
            expression_type("new ArrayList<>()").as_deref(),
            Some("ArrayList")
        );
        assert_eq!(expression_type("foo()"), None);
    }

    #[test]
    fn context_matching_filters_templates() {
        let registry = TemplateRegistry::bundled();
        let java = Path::new("A.java");
        let at = |text: &str| contexts_at(java, text, text.chars().count());
        let body = at("class A {\n  void f() {\n    ");
        let members = at("class A {\n  ");
        assert!(registry.find_in(java, &body, "sout").is_some());
        assert!(registry.find_in(java, &members, "sout").is_none());
        assert!(registry.find_in(java, &members, "psvm").is_some());
        assert!(registry.find_in(java, &body, "psvm").is_none());
        assert!(registry.find_in(java, &body, "St").is_some());
        assert!(
            registry
                .find_in(java, &at("class A {\n  void f() {\n    // "), "todo")
                .is_some()
        );
        assert!(
            registry
                .surround_templates(java, &body)
                .any(|t| t.abbreviation == "T")
        );
        assert!(registry.templates_for(java).all(|t| !t.is_surround()));
    }

    #[test]
    fn user_templates_override_and_disabled_keys_hide() {
        let user = UserTemplate {
            group: "java".into(),
            postfix: false,
            template: Template {
                abbreviation: "sout".into(),
                description: "mine".into(),
                text: "log($END$);".into(),
                ..Default::default()
            },
        };
        let lines = [user.to_line(), "not json".to_owned()];
        let java = Path::new("A.java");
        let registry = TemplateRegistry::bundled()
            .with_user(&lines, ["Kotlin".to_owned(), "Java/inn".to_owned()]);
        assert_eq!(registry.find(java, "sout").unwrap().description, "mine");
        assert_eq!(registry.errors.len(), 1);
        assert!(registry.find(java, "inn").is_none(), "toggled off");
        assert!(
            registry.find(Path::new("a.kt"), "main").is_none(),
            "group toggled off"
        );
        assert!(registry.find(java, "ifn").is_some());
        assert!(UserTemplate::from_line("{\"group\":\"x\",\"template\":{}}").is_err());
    }
}
