// The block format of the acceptance cases (`docs/specs/editor/cases/README.md`, extended by
// `docs/specs/java/cases/README.md`): the parser of `archipel-ide-core/tests/support/
// editor_case_format.rs` without the editor steps (that file also needs `IdeState`).

// ---------------------------------------------------------------------------------------------
// Parsing

#[derive(Debug, Default, Clone)]
struct Case {
    file: String,
    id: String,
    number: u32,
    title: String,
    settings: Vec<(String, String)>,
    unverified: Option<String>,
    templates: Vec<(String, String)>,
    language: String,
    viewport: Option<String>,
    tabs: Option<String>,
    history: Option<String>,
    clipboard: Option<String>,
    before: Option<String>,
    keys: String,
    after: Option<String>,
    after_viewport: Option<String>,
    after_tabs: Option<String>,
    after_font: Option<String>,
}

impl Case {
    fn before_has_folds(&self) -> bool {
        self.before.as_deref().is_some_and(|b| b.contains("<fold "))
    }

    fn name(&self) -> String {
        format!("{} case {}", self.id, self.number)
    }
}


fn parse_file(path: &Path) -> Vec<Case> {
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let source = std::fs::read_to_string(path).unwrap();
    let mut cases: Vec<Case> = Vec::new();
    let mut fence: Option<(String, String)> = None;
    for line in source.lines() {
        if let Some((kind, body)) = &mut fence {
            if line == "```" {
                let (kind, body) = (kind.clone(), body.clone());
                fence = None;
                let case = cases.last_mut().unwrap();
                let body = body.strip_suffix('\n').unwrap_or(&body).to_owned();
                match kind.as_str() {
                    "before" => case.before = Some(body),
                    "after" => case.after = Some(body),
                    _ => {}
                }
            } else {
                body.push_str(line);
                body.push('\n');
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            let (head, title) = rest.split_once(": ").unwrap_or((rest, ""));
            let (id, number) = head.split_once(" case ").unwrap_or((head, "0"));
            cases.push(Case {
                file: name.clone(),
                id: id.trim().to_owned(),
                number: number.trim().parse().unwrap_or(0),
                title: title.trim().to_owned(),
                ..Case::default()
            });
            continue;
        }
        let Some(case) = cases.last_mut() else {
            continue;
        };
        if let Some(kind) = line.strip_prefix("```") {
            fence = Some((kind.trim().to_owned(), String::new()));
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_owned();
        match key {
            "settings" if value != "default" => {
                case.settings = split_settings(&value);
            }
            "settings" => {}
            "status" => case.unverified = Some(value),
            "template" => {
                if let Some((abbr, text)) = value.split_once(" = ") {
                    case.templates
                        .push((abbr.trim().to_owned(), text.to_owned()));
                }
            }
            "language" => case.language = value,
            "viewport" => case.viewport = Some(value),
            "tabs" => case.tabs = Some(value),
            "history" => case.history = Some(value),
            "clipboard" => case.clipboard = Some(value),
            "keys" => case.keys = value,
            "after viewport" => case.after_viewport = Some(value),
            "after tabs" => case.after_tabs = Some(value),
            "after font size" => case.after_font = Some(value),
            _ => {}
        }
    }
    cases
}

/// `a=on, b=Text with, commas?` is split at `, ` followed by something that looks like `name=`.
fn split_settings(value: &str) -> Vec<(String, String)> {
    let mut parts: Vec<String> = Vec::new();
    for piece in value.split(", ") {
        if piece.contains('=') || parts.is_empty() {
            parts.push(piece.to_owned());
        } else if let Some(last) = parts.last_mut() {
            last.push_str(", ");
            last.push_str(piece);
        }
    }
    parts
        .into_iter()
        .filter_map(|part| {
            let (name, value) = part.rsplit_once('=')?;
            Some((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FoldMark {
    start: usize,
    end: usize,
    placeholder: String,
    expanded: bool,
}

/// A text with `<caret>`, `<selection>` and `<fold>` markup removed.
#[derive(Debug, Clone, Default)]
struct Marked {
    text: String,
    /// `(anchor, head)` in character offsets, in the order of the markup.
    selections: Vec<(usize, usize)>,
    folds: Vec<FoldMark>,
}

fn parse_marked(source: &str) -> Marked {
    let source = source.replace("\\t", "\t");
    let mut out = Marked::default();
    let mut count = 0usize;
    let mut selection_start: Option<(usize, Option<usize>)> = None;
    let mut fold_stack: Vec<(usize, String, bool)> = Vec::new();
    let mut rest = source.as_str();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<caret>") {
            match &mut selection_start {
                Some((_, caret)) => *caret = Some(count),
                None => out.selections.push((count, count)),
            }
            rest = after;
        } else if let Some(after) = rest.strip_prefix("<selection>") {
            selection_start = Some((count, None));
            rest = after;
        } else if let Some(after) = rest.strip_prefix("</selection>") {
            let (start, caret) = selection_start.take().expect("closing without opening");
            let selection = match caret {
                Some(caret) if caret == start && caret != count => (count, start),
                Some(caret) => (start, caret),
                None => (start, count),
            };
            out.selections.push(selection);
            rest = after;
        } else if rest.starts_with("<fold ") {
            let end = rest.find('>').expect("unterminated fold tag");
            let tag = &rest[..end];
            let text = tag
                .split("text=\"")
                .nth(1)
                .and_then(|t| t.split('"').next())
                .unwrap_or("...");
            fold_stack.push((count, text.to_owned(), tag.contains(" expanded")));
            rest = &rest[end + 1..];
        } else if let Some(after) = rest.strip_prefix("</fold>") {
            let (start, placeholder, expanded) = fold_stack.pop().expect("closing fold");
            out.folds.push(FoldMark {
                start,
                end: count,
                placeholder,
                expanded,
            });
            rest = after;
        } else {
            let c = rest.chars().next().unwrap();
            out.text.push(c);
            count += 1;
            rest = &rest[c.len_utf8()..];
        }
    }
    out.folds
        .sort_by_key(|f| (f.start, std::cmp::Reverse(f.end)));
    out
}

/// Renders text, selections and folds back to the markup (for failure messages).
fn render_marked(text: &str, selections: &[(usize, usize)], folds: &[FoldMark]) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut inserts: Vec<(usize, u8, String)> = Vec::new();
    for fold in folds {
        let open = if fold.expanded {
            format!("<fold text=\"{}\" expanded>", fold.placeholder)
        } else {
            format!("<fold text=\"{}\">", fold.placeholder)
        };
        inserts.push((fold.start, 2, open));
        inserts.push((fold.end, 0, "</fold>".into()));
    }
    for &(anchor, head) in selections {
        if anchor == head {
            inserts.push((head, 3, "<caret>".into()));
        } else if anchor < head {
            inserts.push((anchor, 1, "<selection>".into()));
            inserts.push((head, 1, "<caret></selection>".into()));
        } else {
            inserts.push((head, 1, "<selection><caret>".into()));
            inserts.push((anchor, 1, "</selection>".into()));
        }
    }
    inserts.sort_by_key(|(offset, order, _)| (*offset, *order));
    let mut out = String::new();
    let mut next = 0;
    for offset in 0..=chars.len() {
        while next < inserts.len() && inserts[next].0 == offset {
            out.push_str(&inserts[next].2);
            next += 1;
        }
        if let Some(c) = chars.get(offset) {
            out.push(*c);
        }
    }
    out.replace('\t', "\\t")
}

fn unescape(quoted: &str) -> String {
    let inner = quoted.trim();
    let inner = inner
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(inner);
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

