use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
};

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_RESULTS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextMatch {
    pub path: PathBuf,
    /// 1-based line of the match.
    pub line: usize,
    /// 1-based character column of the first occurrence on the line.
    pub column: usize,
    /// Length in characters of the first occurrence.
    pub length: usize,
    /// The trimmed line, at most [`PREVIEW_CHARS`] characters. Longer lines
    /// are elided around the first occurrence, with `…` where text was cut.
    pub preview: String,
    /// Character ranges of every occurrence within `preview`.
    pub highlights: Vec<std::ops::Range<usize>>,
}

/// One line changed by a replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineReplacement {
    /// 1-based line number.
    pub line: usize,
    /// The line before and after, trimmed and elided like search results.
    pub before: String,
    pub after: String,
    /// Occurrences replaced on this line.
    pub count: usize,
}

/// The replacement result for one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReplacement {
    /// Project-relative path.
    pub path: PathBuf,
    pub lines: Vec<LineReplacement>,
    /// The complete new file contents (line endings preserved).
    pub new_contents: String,
}

impl FileReplacement {
    pub fn count(&self) -> usize {
        self.lines.iter().map(|line| line.count).sum()
    }
}

/// Longest preview kept per result line (including `…` markers).
pub const PREVIEW_CHARS: usize = 120;
/// Characters kept before the first occurrence when a long line is elided.
pub const PREVIEW_CONTEXT: usize = 32;

/// Find in Files options, as in the reference IDE's search popup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchOptions {
    pub match_case: bool,
    pub whole_words: bool,
    /// Treat the query as a regular expression (Rust `regex` syntax).
    pub regex: bool,
    /// Comma-separated globs such as `*.java, *.kt`; `!pattern` excludes.
    /// Patterns without `/` match the file name, others the project path.
    pub file_mask: String,
}

/// A file-mask filter on its own: the body of a user-defined search scope
/// (same syntax as [`SearchOptions::file_mask`]; an empty mask admits everything).
pub struct FileMaskFilter(Matcher);

impl FileMaskFilter {
    pub fn new(mask: &str) -> Result<Self, SearchError> {
        let options = SearchOptions {
            file_mask: mask.to_owned(),
            ..SearchOptions::default()
        };
        Matcher::new("-", &options).map(Self)
    }

    /// Whether the project-relative `path` is in the scope.
    pub fn accepts(&self, path: &Path) -> bool {
        self.0.accepts(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SearchError {
    #[error("search query cannot be empty")]
    EmptyQuery,
    #[error("invalid regular expression: {0}")]
    InvalidRegex(String),
    #[error("invalid file mask `{0}`")]
    InvalidMask(String),
    #[error("failed to start search: {0}")]
    Spawn(String),
}

/// Compiled query and file mask.
#[derive(Debug, Clone)]
pub struct Matcher {
    regex: regex::Regex,
    /// Plain-text queries replace literally; regex queries expand `$1`/`${name}`.
    expand_groups: bool,
    include_names: Option<globset::GlobSet>,
    include_paths: Option<globset::GlobSet>,
    exclude_names: Option<globset::GlobSet>,
    exclude_paths: Option<globset::GlobSet>,
}

impl Matcher {
    pub fn new(query: &str, options: &SearchOptions) -> Result<Self, SearchError> {
        if query.is_empty() {
            return Err(SearchError::EmptyQuery);
        }
        let mut pattern = if options.regex {
            query.to_owned()
        } else {
            regex::escape(query)
        };
        if options.whole_words {
            pattern = format!(r"\b(?:{pattern})\b");
        }
        let regex = regex::RegexBuilder::new(&pattern)
            .case_insensitive(!options.match_case)
            .size_limit(1 << 22)
            .build()
            .map_err(|error| SearchError::InvalidRegex(error.to_string()))?;
        let (mut include_names, mut include_paths) = (Vec::new(), Vec::new());
        let (mut exclude_names, mut exclude_paths) = (Vec::new(), Vec::new());
        for raw in options.file_mask.split([',', ';']) {
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            let (exclude, glob) = match raw.strip_prefix('!') {
                Some(rest) => (true, rest.trim()),
                None => (false, raw),
            };
            // Masks are case-insensitive and match the whole name (SRCH-FIP-020).
            let compiled = globset::GlobBuilder::new(glob)
                .case_insensitive(true)
                .build()
                .map_err(|_| SearchError::InvalidMask(raw.to_owned()))?;
            match (exclude, glob.contains('/')) {
                (false, false) => include_names.push(compiled),
                (false, true) => include_paths.push(compiled),
                (true, false) => exclude_names.push(compiled),
                (true, true) => exclude_paths.push(compiled),
            }
        }
        let set = |globs: Vec<globset::Glob>| -> Result<Option<globset::GlobSet>, SearchError> {
            if globs.is_empty() {
                return Ok(None);
            }
            let mut builder = globset::GlobSetBuilder::new();
            for glob in globs {
                builder.add(glob);
            }
            builder
                .build()
                .map(Some)
                .map_err(|error| SearchError::InvalidMask(error.to_string()))
        };
        Ok(Self {
            regex,
            expand_groups: options.regex,
            include_names: set(include_names)?,
            include_paths: set(include_paths)?,
            exclude_names: set(exclude_names)?,
            exclude_paths: set(exclude_paths)?,
        })
    }

    /// Whether the file mask admits `relative` (always true without a mask).
    pub fn accepts(&self, relative: &Path) -> bool {
        let name = relative.file_name().map(Path::new).unwrap_or(relative);
        let hit = |set: &Option<globset::GlobSet>, candidate: &Path| {
            set.as_ref().is_some_and(|set| set.is_match(candidate))
        };
        if hit(&self.exclude_names, name) || hit(&self.exclude_paths, relative) {
            return false;
        }
        if self.include_names.is_none() && self.include_paths.is_none() {
            return true;
        }
        hit(&self.include_names, name) || hit(&self.include_paths, relative)
    }

    /// `line` with every non-empty match replaced, and how many were replaced;
    /// `None` when nothing matched. Regex queries expand `$1`, `${name}` and `$$`.
    pub fn replace_line(&self, line: &str, replacement: &str) -> Option<(String, usize)> {
        let mut out = String::with_capacity(line.len());
        let mut last = 0;
        let mut count = 0;
        for captures in self.regex.captures_iter(line) {
            let Some(whole) = captures.get(0).filter(|whole| !whole.is_empty()) else {
                continue;
            };
            out.push_str(&line[last..whole.start()]);
            if self.expand_groups {
                captures.expand(replacement, &mut out);
            } else {
                out.push_str(replacement);
            }
            last = whole.end();
            count += 1;
        }
        if count == 0 {
            return None;
        }
        out.push_str(&line[last..]);
        Some((out, count))
    }

    /// Replaces matches in `contents` line by line (as search finds them),
    /// preserving each line's ending. `None` when nothing matched.
    pub fn replace_contents(
        &self,
        path: &Path,
        contents: &str,
        replacement: &str,
    ) -> Option<FileReplacement> {
        let mut lines = Vec::new();
        let mut new_contents = String::with_capacity(contents.len());
        for (index, raw) in contents.split_inclusive('\n').enumerate() {
            let body = raw.trim_end_matches(['\n', '\r']);
            let ending = &raw[body.len()..];
            match self.replace_line(body, replacement) {
                Some((after, count)) => {
                    lines.push(LineReplacement {
                        line: index + 1,
                        before: elide_around(body.trim(), 0).0,
                        after: elide_around(after.trim(), 0).0,
                        count,
                    });
                    new_contents.push_str(&after);
                    new_contents.push_str(ending);
                }
                None => new_contents.push_str(raw),
            }
        }
        (!lines.is_empty()).then(|| FileReplacement {
            path: path.to_owned(),
            lines,
            new_contents,
        })
    }

    /// Non-empty matches in `line`, as character ranges.
    pub fn occurrences(&self, line: &str) -> Vec<std::ops::Range<usize>> {
        let mut ranges = Vec::new();
        // Regex offsets are bytes; results are character columns.
        let boundaries: Vec<usize> = line.char_indices().map(|(byte, _)| byte).collect();
        let char_at = |byte: usize| boundaries.partition_point(|&b| b < byte);
        for found in self.regex.find_iter(line) {
            if found.is_empty() {
                continue;
            }
            let start = char_at(found.start());
            let end = char_at(found.end());
            ranges.push(start..end);
        }
        ranges
    }
}

/// At most [`PREVIEW_CHARS`] characters of `text` around character `focus`:
/// short text is kept whole; otherwise a window starting [`PREVIEW_CONTEXT`]
/// characters before `focus`, with `…` replacing the cut parts. Returns the
/// preview and the shift from `text` columns to preview columns.
fn elide_around(text: &str, focus: usize) -> (String, isize) {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= PREVIEW_CHARS {
        return (text.to_owned(), 0);
    }
    // Reserve room for both markers, then slide the window to the end if needed.
    let room = PREVIEW_CHARS - 2;
    let from = focus
        .saturating_sub(PREVIEW_CONTEXT)
        .min(chars.len() - room);
    let to = (from + room).min(chars.len());
    let mut preview = String::new();
    let mut shift = -(from as isize);
    if from > 0 {
        preview.push('…');
        shift += 1;
    }
    preview.extend(&chars[from..to]);
    if to < chars.len() {
        preview.push('…');
    }
    (preview, shift)
}

/// Builds the result row for one line, or `None` when nothing matches.
pub fn line_match(
    path: &Path,
    line_index: usize,
    line: &str,
    matcher: &Matcher,
) -> Option<TextMatch> {
    let ranges = matcher.occurrences(line);
    let first = ranges.first()?.clone();
    let leading = line.chars().take_while(|c| c.is_whitespace()).count();
    let (preview, offset) = elide_around(line.trim(), first.start.saturating_sub(leading));
    let preview_len = preview.chars().count();
    let highlights = ranges
        .iter()
        .filter_map(|range| {
            let shift = |at: usize| {
                (at.saturating_sub(leading) as isize + offset).clamp(0, preview_len as isize)
                    as usize
            };
            let (start, end) = (shift(range.start), shift(range.end));
            (start < end).then_some(start..end)
        })
        .collect();
    Some(TextMatch {
        path: path.to_owned(),
        line: line_index + 1,
        column: first.start + 1,
        length: first.end - first.start,
        preview,
        highlights,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchEvent {
    Match(TextMatch),
    Finished {
        job: u64,
        files_scanned: usize,
        truncated: bool,
    },
}

pub struct SearchService {
    generation: Arc<AtomicU64>,
    receiver: Option<Receiver<SearchEvent>>,
    active_job: Option<u64>,
}

impl Default for SearchService {
    fn default() -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(0)),
            receiver: None,
            active_job: None,
        }
    }
}

impl SearchService {
    pub fn start(
        &mut self,
        root: PathBuf,
        files: Vec<PathBuf>,
        query: String,
        options: &SearchOptions,
    ) -> Result<u64, SearchError> {
        let matcher = Matcher::new(&query, options)?;
        let job = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = Arc::clone(&self.generation);
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.active_job = Some(job);
        let spawn_result = thread::Builder::new()
            .name(format!("archipel-ide-search-{job}"))
            .spawn(move || {
                let mut files_scanned = 0;
                let mut result_count = 0;
                let mut truncated = false;
                for relative_path in files {
                    if generation.load(Ordering::SeqCst) != job {
                        return;
                    }
                    if !matcher.accepts(&relative_path) {
                        continue;
                    }
                    let path = root.join(&relative_path);
                    let Ok(metadata) = fs::metadata(&path) else {
                        continue;
                    };
                    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
                        continue;
                    }
                    let Ok(contents) = fs::read_to_string(path) else {
                        continue;
                    };
                    files_scanned += 1;
                    for (line_index, line) in contents.lines().enumerate() {
                        if generation.load(Ordering::Relaxed) != job {
                            return;
                        }
                        // One row per line, however many occurrences it has.
                        let Some(found) = line_match(&relative_path, line_index, line, &matcher)
                        else {
                            continue;
                        };
                        if sender.send(SearchEvent::Match(found)).is_err() {
                            return;
                        }
                        result_count += 1;
                        if result_count >= MAX_RESULTS {
                            truncated = true;
                            break;
                        }
                    }
                    if truncated {
                        break;
                    }
                }
                let _ = sender.send(SearchEvent::Finished {
                    job,
                    files_scanned,
                    truncated,
                });
            });
        if let Err(error) = spawn_result {
            self.receiver = None;
            self.active_job = None;
            return Err(SearchError::Spawn(error.to_string()));
        }
        Ok(job)
    }

    pub fn cancel(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.receiver = None;
        self.active_job = None;
    }

    pub fn poll(&mut self) -> Vec<SearchEvent> {
        let Some(receiver) = &self.receiver else {
            return Vec::new();
        };
        let events: Vec<_> = receiver.try_iter().collect();
        if events
            .iter()
            .any(|event| matches!(event, SearchEvent::Finished { .. }))
        {
            self.active_job = None;
        }
        events
    }

    pub fn is_running(&self) -> bool {
        self.active_job.is_some()
    }
}

impl Drop for SearchService {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

/// Computes the replacement for each accepted file of `files` without writing
/// anything (the preview of Replace in Files). Unreadable, oversized and
/// non-UTF-8 files are skipped.
pub fn plan_replacements(
    root: &Path,
    files: &[PathBuf],
    matcher: &Matcher,
    replacement: &str,
) -> Vec<FileReplacement> {
    files
        .iter()
        .filter(|relative| matcher.accepts(relative))
        .filter_map(|relative| {
            let path = root.join(relative);
            let metadata = fs::metadata(&path).ok()?;
            if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
                return None;
            }
            let contents = fs::read_to_string(path).ok()?;
            matcher.replace_contents(relative, &contents, replacement)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn file_mask_filters_define_scopes() {
        let scope = super::FileMaskFilter::new("*.rs, src/api/**, !**/generated/**").unwrap();
        assert!(scope.accepts(std::path::Path::new("crates/a/lib.rs")));
        assert!(scope.accepts(std::path::Path::new("src/api/v1/x.txt")));
        assert!(!scope.accepts(std::path::Path::new("docs/readme.md")));
        assert!(!scope.accepts(std::path::Path::new("a/generated/b.rs")));
        assert!(
            super::FileMaskFilter::new("")
                .unwrap()
                .accepts("any/file".as_ref())
        );
        assert!(super::FileMaskFilter::new("[").is_err());
    }

    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn streams_case_insensitive_matches_and_completion() {
        let root = std::env::temp_dir().join(format!("archipel-ide-search-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("one.rs"), "fn Archipel() {}\n// archipel\n").unwrap();
        let mut service = SearchService::default();
        service
            .start(
                root.clone(),
                vec![PathBuf::from("one.rs")],
                "archipel".into(),
                &SearchOptions::default(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut events = Vec::new();
        while Instant::now() < deadline {
            events.extend(service.poll());
            if !service.is_running() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, SearchEvent::Match(_)))
                .count(),
            2
        );
        assert!(events.iter().any(|event| matches!(
            event,
            SearchEvent::Finished {
                files_scanned: 1,
                ..
            }
        )));
        fs::remove_dir_all(root).unwrap();
    }

    fn matcher(query: &str, options: SearchOptions) -> Matcher {
        Matcher::new(query, &options).unwrap()
    }

    #[test]
    fn replacements_expand_groups_only_in_regex_mode_and_keep_line_endings() {
        let regex = matcher(
            r"let (\w+) = (\d+)",
            SearchOptions {
                regex: true,
                match_case: true,
                ..Default::default()
            },
        );
        let plan = regex
            .replace_contents(
                Path::new("a.kt"),
                "let a = 1;\r\nkeep\r\nlet b = 2; let c = 3;",
                "val $1: Int = $2",
            )
            .unwrap();
        assert_eq!(
            plan.new_contents,
            "val a: Int = 1;\r\nkeep\r\nval b: Int = 2; val c: Int = 3;"
        );
        assert_eq!(plan.count(), 3);
        assert_eq!(plan.lines[1].line, 3);
        assert_eq!(plan.lines[0].before, "let a = 1;");
        assert_eq!(plan.lines[0].after, "val a: Int = 1;");
        let literal = matcher("a.b", SearchOptions::default());
        let (text, count) = literal.replace_line("a.b axb", "$1").unwrap();
        assert_eq!((text.as_str(), count), ("$1 axb", 1), "literal, no groups");
        assert!(literal.replace_line("nothing", "x").is_none());
        let empty = matcher(
            "x*",
            SearchOptions {
                regex: true,
                ..Default::default()
            },
        );
        assert!(
            empty.replace_line("abc", "-").is_none(),
            "empty matches skipped"
        );
    }

    #[test]
    fn a_line_with_two_hits_is_one_row_with_two_highlights() {
        let found = line_match(
            Path::new("a.kt"),
            4,
            "    val greeting = greet(name)",
            &matcher("greet", SearchOptions::default()),
        )
        .unwrap();
        assert_eq!(found.line, 5);
        assert_eq!(found.column, 9, "first hit, 1-based, before trimming");
        assert_eq!(found.length, 5);
        assert_eq!(found.preview, "val greeting = greet(name)");
        assert_eq!(found.highlights, vec![4..9, 15..20]);
        let none = matcher("greet", SearchOptions::default());
        assert!(line_match(Path::new("a.kt"), 0, "nothing here", &none).is_none());
    }

    #[test]
    fn long_lines_are_elided_around_the_match() {
        let matcher = Matcher::new("needle", &SearchOptions::default()).unwrap();
        let line = format!("  {}needle{}", "a".repeat(5000), "b".repeat(5000));
        let found = line_match(Path::new("min.js"), 0, &line, &matcher).unwrap();
        assert!(found.preview.chars().count() <= PREVIEW_CHARS);
        assert!(found.preview.starts_with('…') && found.preview.ends_with('…'));
        assert_eq!(found.highlights.len(), 1);
        let chars: Vec<char> = found.preview.chars().collect();
        let range = found.highlights[0].clone();
        assert_eq!(chars[range.clone()].iter().collect::<String>(), "needle");
        assert_eq!(
            range.start,
            1 + PREVIEW_CONTEXT,
            "context kept before the match"
        );
        // The column still points into the full line.
        assert_eq!(found.column, 2 + 5000 + 1);
        // A match near the end slides the window instead of leaving it short.
        let tail = format!("{}needle", "c".repeat(5000));
        let found = line_match(Path::new("t"), 0, &tail, &matcher).unwrap();
        assert!(found.preview.starts_with('…') && found.preview.ends_with("needle"));
        assert_eq!(found.preview.chars().count(), PREVIEW_CHARS - 1);
        // Short lines are unchanged.
        let found = line_match(Path::new("t"), 0, "  one needle  ", &matcher).unwrap();
        assert_eq!(found.preview, "one needle");
        assert_eq!(found.highlights, vec![4..10]);
    }

    #[test]
    fn multibyte_text_maps_to_character_columns() {
        let jet = matcher("jet", SearchOptions::default());
        assert_eq!(jet.occurrences("İx jet JET"), vec![3..6, 7..10]);
        assert_eq!(jet.occurrences("ä jet"), vec![2..5]);
        assert_eq!(
            matcher("aa", SearchOptions::default()).occurrences("aaaa"),
            vec![0..2, 2..4]
        );
    }

    #[test]
    fn match_case_whole_words_and_regex_options() {
        let line = "greet Greet greeting";
        let plain = matcher("greet", SearchOptions::default());
        assert_eq!(plain.occurrences(line).len(), 3);
        let case = matcher(
            "greet",
            SearchOptions {
                match_case: true,
                ..Default::default()
            },
        );
        assert_eq!(case.occurrences(line), vec![0..5, 12..17]);
        let words = matcher(
            "greet",
            SearchOptions {
                whole_words: true,
                ..Default::default()
            },
        );
        assert_eq!(words.occurrences(line), vec![0..5, 6..11]);
        let regex = matcher(
            r"gr\w+ing",
            SearchOptions {
                regex: true,
                ..Default::default()
            },
        );
        assert_eq!(regex.occurrences(line), vec![12..20]);
        // Regex metacharacters are literal unless regex mode is on.
        assert!(
            matcher("a.c", SearchOptions::default())
                .occurrences("abc")
                .is_empty()
        );
        // Empty matches never produce rows.
        let empty = matcher(
            "x*",
            SearchOptions {
                regex: true,
                ..Default::default()
            },
        );
        assert!(empty.occurrences("abc").is_empty());
        assert!(matches!(
            Matcher::new(
                "(",
                &SearchOptions {
                    regex: true,
                    ..Default::default()
                }
            ),
            Err(SearchError::InvalidRegex(_))
        ));
        assert_eq!(
            Matcher::new("", &SearchOptions::default()).unwrap_err(),
            SearchError::EmptyQuery
        );
    }

    #[test]
    fn file_masks_include_and_exclude() {
        let mask = |mask: &str| {
            matcher(
                "x",
                SearchOptions {
                    file_mask: mask.into(),
                    ..Default::default()
                },
            )
        };
        let java = mask("*.java, *.kt");
        assert!(java.accepts(Path::new("src/main/java/App.java")));
        assert!(java.accepts(Path::new("app/Main.kt")));
        assert!(!java.accepts(Path::new("build.gradle.kts")));
        let no_tests = mask("*.java; !*Test.java");
        assert!(no_tests.accepts(Path::new("src/App.java")));
        assert!(!no_tests.accepts(Path::new("src/AppTest.java")));
        let by_path = mask("src/test/**");
        assert!(by_path.accepts(Path::new("src/test/java/AppTest.java")));
        assert!(!by_path.accepts(Path::new("src/main/java/App.java")));
        assert!(mask("").accepts(Path::new("anything")));
        assert!(matches!(
            Matcher::new(
                "x",
                &SearchOptions {
                    file_mask: "[".into(),
                    ..Default::default()
                }
            ),
            Err(SearchError::InvalidMask(_))
        ));
    }

    #[test]
    fn new_search_cancels_previous_generation() {
        let mut service = SearchService::default();
        let first = service
            .start(
                PathBuf::from("/tmp"),
                Vec::new(),
                "one".into(),
                &SearchOptions::default(),
            )
            .unwrap();
        let second = service
            .start(
                PathBuf::from("/tmp"),
                Vec::new(),
                "two".into(),
                &SearchOptions::default(),
            )
            .unwrap();
        assert!(second > first);
    }
}
