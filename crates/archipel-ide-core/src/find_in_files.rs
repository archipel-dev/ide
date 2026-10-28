//! The model of the Find in Files popup (spec `docs/specs/search/02-find-in-files.md`, SRCH-FIP):
//! scopes, which files are skipped, the file mask, validation, the result rows and their order,
//! the info label, the empty-result hints and Replace in Files with its confirmation.
//!
//! The model works on an in-memory view of the project ([`ProjectFile`]) so the window, the
//! terminal frontend and the tests share it; the background search of
//! `archipel_ide_search::SearchService` feeds the same rows from disk.

use crate::find_session::{expand_template, preserve_case};
use archipel_ide_search::{Matcher, SearchError, SearchOptions, TextMatch, line_match};
use regex::RegexBuilder;
use std::path::{Path, PathBuf};

/// Result rows loaded per page (`ide.usages.page.size`, SRCH-FIP-042).
pub const PAGE_SIZE: usize = 100;

/// A file of the project as the search sees it.
#[derive(Debug, Clone)]
pub struct ProjectFile {
    /// Project-relative path.
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    /// The folder is marked excluded, ignored, or the file belongs to a library root.
    pub excluded: bool,
}

impl ProjectFile {
    pub fn text(path: &str, text: &str) -> Self {
        Self {
            path: path.into(),
            bytes: text.as_bytes().to_vec(),
            excluded: false,
        }
    }
}

/// The scope row (SRCH-FIP-011).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Project,
    /// A project-relative directory.
    Directory {
        path: PathBuf,
        recursive: bool,
    },
    /// The content root of a module.
    Module {
        name: String,
        root: PathBuf,
    },
}

/// What the popup is asked to find.
#[derive(Debug, Clone)]
pub struct FindRequest {
    pub query: String,
    pub options: SearchOptions,
    pub file_mask_on: bool,
    pub scope: Scope,
}

impl Default for FindRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            options: SearchOptions::default(),
            file_mask_on: false,
            scope: Scope::Project,
        }
    }
}

/// Binary files are skipped (SRCH-FIP-012): a NUL byte in the first 8000 bytes.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8000).any(|b| *b == 0)
}

/// Validates the file mask text (SRCH-FIP-021); `Err` is the message under the field.
pub fn validate_file_mask(mask: &str) -> Result<(), String> {
    if mask.contains(';') {
        return Err("File masks should be comma-separated".into());
    }
    if mask.split(',').all(|entry| entry.trim().is_empty()) {
        return Err("Empty file mask".into());
    }
    for entry in mask.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        if archipel_ide_search::FileMaskFilter::new(entry).is_err() {
            return Err(format!("Bad file mask \"{entry}\""));
        }
    }
    Ok(())
}

/// Validates the request before any search runs (SRCH-FIP-030). `directory_exists` answers for a
/// directory scope.
pub fn validate(
    request: &FindRequest,
    directory_exists: impl Fn(&Path) -> bool,
) -> Result<(), String> {
    if request.file_mask_on {
        validate_file_mask(&request.options.file_mask)?;
    }
    if request.query.is_empty() && !request.file_mask_on {
        return Err("Type search query to find in files".into());
    }
    if request.options.regex && !request.query.is_empty() {
        let regex = RegexBuilder::new(&request.query).build().map_err(|e| {
            let reason = e.to_string();
            let reason = reason
                .lines()
                .last()
                .unwrap_or("")
                .trim_start_matches("error: ");
            format!("Bad pattern \"{}\": {reason}", request.query)
        })?;
        if regex.is_match("") && !request.query.starts_with('^') && !request.query.ends_with('$') {
            return Err("Regular expression matches empty string".into());
        }
    }
    if let Scope::Directory { path, .. } = &request.scope
        && !directory_exists(path)
    {
        return Err("Directory is not found".into());
    }
    Ok(())
}

fn in_scope(path: &Path, scope: &Scope) -> bool {
    match scope {
        Scope::Project => true,
        Scope::Directory {
            path: dir,
            recursive,
        } => match path.strip_prefix(dir) {
            Ok(rest) => *recursive || rest.components().count() == 1,
            Err(_) => false,
        },
        Scope::Module { root, .. } => path.starts_with(root),
    }
}

/// The result of one run.
#[derive(Debug, Clone, Default)]
pub struct FindResults {
    /// Rows in display order.
    pub rows: Vec<TextMatch>,
    /// Occurrences (a row may hold several).
    pub occurrences: usize,
    pub files: usize,
}

impl FindResults {
    /// The info label (SRCH-FIP-040).
    pub fn label(&self) -> String {
        let plural = |n: usize, one: &str, many: &str| {
            if n == 1 {
                format!("{n} {one}")
            } else {
                format!("{n} {many}")
            }
        };
        format!(
            "{} in {}",
            plural(self.occurrences, "match", "matches"),
            plural(self.files, "file", "files")
        )
    }
}

/// Runs the search over `files`. `open` are the unsaved texts of the open editors, in the order
/// they were opened; their files come first (SRCH-FIP-041, 043).
pub fn find(
    files: &[ProjectFile],
    open: &[(PathBuf, String)],
    request: &FindRequest,
) -> Result<FindResults, SearchError> {
    let mut options = request.options.clone();
    if !request.file_mask_on {
        options.file_mask.clear();
    }
    let query = if request.query.is_empty() && request.file_mask_on {
        // A mask-only search lists nothing but is allowed.
        return Ok(FindResults::default());
    } else {
        request.query.as_str()
    };
    let matcher = Matcher::new(query, &options)?;
    let mut rows: Vec<TextMatch> = Vec::new();
    let mut occurrences = 0;
    let mut hit_files: Vec<PathBuf> = Vec::new();
    for file in files {
        if file.excluded || !in_scope(&file.path, &request.scope) || !matcher.accepts(&file.path) {
            continue;
        }
        let text = match open.iter().find(|(p, _)| *p == file.path) {
            Some((_, text)) => text.clone(),
            None => {
                if is_binary(&file.bytes) {
                    continue;
                }
                match String::from_utf8(file.bytes.clone()) {
                    Ok(t) => t,
                    Err(_) => continue,
                }
            }
        };
        let mut found = false;
        for (index, line) in text.lines().enumerate() {
            if let Some(row) = line_match(&file.path, index, line, &matcher) {
                occurrences += row.highlights.len().max(1);
                rows.push(row);
                found = true;
            }
        }
        if found {
            hit_files.push(file.path.clone());
        }
    }
    let open_rank = |p: &Path| open.iter().position(|(o, _)| o == p);
    rows.sort_by(|a, b| {
        let (oa, ob) = (open_rank(&a.path), open_rank(&b.path));
        match (oa, ob) {
            (Some(x), Some(y)) if x != y => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            _ => a
                .path
                .to_string_lossy()
                .cmp(&b.path.to_string_lossy())
                .then(a.line.cmp(&b.line))
                .then(a.column.cmp(&b.column)),
        }
    });
    Ok(FindResults {
        rows,
        occurrences,
        files: hit_files.len(),
    })
}

/// The preview of a row with the matches in square brackets (`// [needle]`).
pub fn bracketed(row: &TextMatch) -> String {
    let chars: Vec<char> = row.preview.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    for range in &row.highlights {
        out.extend(&chars[at..range.start]);
        out.push('[');
        out.extend(&chars[range.start..range.end]);
        out.push(']');
        at = range.end;
    }
    out.extend(&chars[at..]);
    out
}

/// The text of an empty result list (SRCH-FIP-046); lines are separated by `\n`.
pub fn empty_text(request: &FindRequest) -> String {
    let regex_off = !request.options.regex;
    let has_meta = request.query.contains([
        '.', '$', '|', '(', ')', '[', ']', '{', '}', '^', '?', '*', '+', '\\',
    ]);
    if regex_off && has_meta && RegexBuilder::new(&request.query).build().is_ok() {
        return "Nothing found.\nSearch with Regex Alt+X".into();
    }
    let mut used: Vec<&str> = Vec::new();
    if request.options.match_case {
        used.push("Match case");
    }
    if request.options.whole_words {
        used.push("Words");
    }
    if request.options.regex {
        used.push("Regex");
    }
    if request.file_mask_on {
        used.push("File mask");
    }
    let mut text = match used.len() {
        0 => "Nothing found".to_owned(),
        1 => format!(
            "Nothing found.\nSearch option used: {}\nClear option Alt+Shift+Backspace",
            used[0]
        ),
        _ => format!(
            "Nothing found.\nSearch options used:\n{}\nClear all options Alt+Shift+Backspace",
            used.join("  ")
        ),
    };
    if let Scope::Directory {
        recursive: false, ..
    } = request.scope
    {
        text = if used.is_empty() {
            "Nothing found. Search recursively in subdirectories".to_owned()
        } else {
            format!("{text}\nSearch recursively in subdirectories")
        };
    }
    text
}

/// The confirmation of Replace All over several files (SRCH-FIP-071); `None` when the matches
/// are in one file or there is one row.
pub fn replace_confirmation(
    results: &FindResults,
    find: &str,
    replacement: &str,
) -> Option<(String, String)> {
    if results.rows.len() <= 1 || results.files <= 1 {
        return None;
    }
    Some((
        "Replace All".into(),
        format!(
            "Replace {} occurrences of '{find}' across {} files with '{replacement}'?",
            results.occurrences, results.files
        ),
    ))
}

/// The outcome of Replace All over the scope.
#[derive(Debug, Clone, Default)]
pub struct ReplaceOutcome {
    /// New contents per changed file.
    pub changed: Vec<(PathBuf, String)>,
    pub replaced: usize,
    pub status: String,
    /// `Replace Error` dialog text when the template is malformed for the pattern.
    pub error: Option<String>,
    /// The notification of SRCH-FIP-073.
    pub notification: Option<String>,
    /// The Local History label (SRCH-FIP-072).
    pub history_label: String,
}

/// Replace All (SRCH-FIP-071, 072, 073): every match of the scope, left to right per file.
pub fn replace_all(
    files: &[ProjectFile],
    open: &[(PathBuf, String)],
    request: &FindRequest,
    replacement: &str,
    preserve: bool,
) -> Result<ReplaceOutcome, SearchError> {
    let mut options = request.options.clone();
    if !request.file_mask_on {
        options.file_mask.clear();
    }
    let matcher = Matcher::new(&request.query, &options)?;
    let regex = options
        .regex
        .then(|| {
            RegexBuilder::new(&if options.whole_words {
                format!(r"\b(?:{})\b", request.query)
            } else {
                request.query.clone()
            })
            .case_insensitive(!options.match_case)
            .multi_line(true)
            .build()
        })
        .transpose()
        .map_err(|e| SearchError::InvalidRegex(e.to_string()))?;
    let scope_label = match &request.scope {
        Scope::Project => "Project".to_owned(),
        Scope::Module { name, .. } => format!("Module '{name}'"),
        Scope::Directory { path, .. } => format!("Directory {}", path.display()),
    };
    let mut out = ReplaceOutcome {
        history_label: format!(
            "Replace all '{}' in {scope_label} with '{replacement}'",
            request.query
        ),
        ..Default::default()
    };
    // Dry run on the first match: the template against the group count (SRCH-FIP-031).
    if let Some(regex) = &regex {
        let mut dry = None;
        for file in files {
            if file.excluded
                || !in_scope(&file.path, &request.scope)
                || !matcher.accepts(&file.path)
                || is_binary(&file.bytes)
            {
                continue;
            }
            if let Ok(text) = std::str::from_utf8(&file.bytes)
                && let Some(caps) = regex.captures(text)
            {
                dry = Some(expand_dry(replacement, regex, &caps));
                break;
            }
        }
        if let Some(Err(message)) = dry {
            out.error = Some(format!("Malformed replacement string: {message}"));
            return Ok(out);
        }
    }
    let mut malformed = false;
    for file in files {
        if file.excluded || !in_scope(&file.path, &request.scope) || !matcher.accepts(&file.path) {
            continue;
        }
        let text = match open.iter().find(|(p, _)| *p == file.path) {
            Some((_, t)) => t.clone(),
            None if is_binary(&file.bytes) => continue,
            None => match String::from_utf8(file.bytes.clone()) {
                Ok(t) => t,
                Err(_) => continue,
            },
        };
        let mut new = String::with_capacity(text.len());
        let mut last = 0;
        let mut count = 0;
        let ranges: Vec<std::ops::Range<usize>> = match &regex {
            Some(re) => re
                .find_iter(&text)
                .filter(|m| !m.is_empty())
                .map(|m| m.range())
                .collect(),
            None => {
                let literal = RegexBuilder::new(&regex::escape(&request.query))
                    .case_insensitive(!options.match_case)
                    .build()
                    .map_err(|e| SearchError::InvalidRegex(e.to_string()))?;
                literal.find_iter(&text).map(|m| m.range()).collect()
            }
        };
        for range in ranges {
            let found = &text[range.clone()];
            let mut replaced = match &regex {
                Some(re) => {
                    let caps = re
                        .captures(&text[range.start..])
                        .filter(|c| c.get(0).is_some_and(|m| m.start() == 0));
                    // Captures are taken at the match start; anchors need the whole text.
                    let caps = caps.or_else(|| {
                        re.captures_iter(&text)
                            .find(|c| c.get(0).is_some_and(|m| m.start() == range.start))
                    });
                    match caps.map(|c| expand_template(replacement, re, &c)) {
                        Some(Ok(s)) => s,
                        _ => {
                            malformed = true;
                            continue;
                        }
                    }
                }
                None => replacement.to_owned(),
            };
            if preserve {
                replaced = preserve_case(found, &replaced);
            }
            new.push_str(&text[last..range.start]);
            new.push_str(&replaced);
            last = range.end;
            count += 1;
        }
        if count > 0 {
            new.push_str(&text[last..]);
            out.replaced += count;
            out.changed.push((file.path.clone(), new));
        }
    }
    out.status = format!("{} occurrences replaced", out.replaced);
    if malformed {
        out.notification = Some("One or more malformed replacement strings".into());
    }
    Ok(out)
}

/// The dry run checks the group count only; group names are not checked (SRCH-FIP-031).
fn expand_dry(
    template: &str,
    regex: &regex::Regex,
    caps: &regex::Captures,
) -> Result<String, String> {
    let sanitized = strip_named_groups(template);
    expand_template(&sanitized, regex, caps)
}

fn strip_named_groups(template: &str) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(at) = rest.find("${") {
        out.push_str(&rest[..at]);
        match rest[at..].find('}') {
            Some(end) => rest = &rest[at + end + 1..],
            None => {
                out.push_str(&rest[at..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_count_matches_and_files() {
        // SRCH-FIP-040
        let files = [
            ProjectFile::text("A.java", "needle needle"),
            ProjectFile::text("B.java", "x"),
            ProjectFile::text("C.java", "needle"),
        ];
        let request = FindRequest {
            query: "needle".into(),
            ..Default::default()
        };
        let found = find(&files, &[], &request).unwrap();
        assert_eq!(found.label(), "3 matches in 2 files");
    }

    #[test]
    fn file_masks_are_validated() {
        // SRCH-FIP-021
        assert_eq!(
            validate_file_mask("*.java;*.kt").unwrap_err(),
            "File masks should be comma-separated"
        );
        assert_eq!(validate_file_mask("").unwrap_err(), "Empty file mask");
    }
}
