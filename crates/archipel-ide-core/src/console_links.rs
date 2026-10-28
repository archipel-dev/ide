//! Links in console output: `https://` URLs and `path/File.ext:line[:column]`
//! locations (IntelliJ's console filters), shared by the window and the terminal frontend, and
//! the resolution of a printed location to a project file.

use crate::IdeState;
use std::ops::Range;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Url(String),
    File {
        path: String,
        line: u32,
        column: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// Byte range in the line.
    pub range: Range<usize>,
    pub target: LinkTarget,
}

fn is_url_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '<' | '>' | '"' | '\'' | '`' | '|')
}

fn is_path_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '/' | '\\' | '.' | '_' | '-' | '+' | '@' | '~')
}

fn count(text: &str, c: char) -> usize {
    text.matches(c).count()
}

/// Links of one console line, left to right and disjoint.
pub fn find(text: &str) -> Vec<Link> {
    let mut links = Vec::new();
    let mut from = 0;
    while let Some(found) = ["https://", "http://"]
        .iter()
        .filter_map(|scheme| text[from..].find(scheme))
        .min()
    {
        let start = from + found;
        let end = text[start..]
            .find(|c: char| !is_url_char(c))
            .map_or(text.len(), |i| start + i);
        let mut url = &text[start..end];
        // Trailing punctuation belongs to the sentence, an unbalanced closing
        // bracket does too.
        loop {
            let trimmed = url.trim_end_matches(['.', ',', ';', ':', '!', '?']);
            let trimmed = match trimmed.chars().last() {
                Some(')') if count(trimmed, ')') > count(trimmed, '(') => {
                    &trimmed[..trimmed.len() - 1]
                }
                Some(']') if count(trimmed, ']') > count(trimmed, '[') => {
                    &trimmed[..trimmed.len() - 1]
                }
                _ => trimmed,
            };
            if trimmed.len() == url.len() {
                break;
            }
            url = trimmed;
        }
        if url.len() > "https://".len() {
            links.push(Link {
                range: start..start + url.len(),
                target: LinkTarget::Url(url.to_owned()),
            });
        }
        from = end.max(start + 1);
    }
    let taken = |range: &Range<usize>, links: &[Link]| {
        links
            .iter()
            .any(|link| link.range.start < range.end && range.start < link.range.end)
    };
    let mut files = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        if bytes[i] != b':' {
            i += 1;
            continue;
        }
        // `<path>.<ext>` directly before the colon, digits after it.
        let digits = |at: usize| {
            text[at..]
                .find(|c: char| !c.is_ascii_digit())
                .map_or(text.len(), |n| at + n)
        };
        let line_end = digits(i + 1);
        let path_start = text[..i]
            .char_indices()
            .rev()
            .take_while(|&(_, c)| is_path_char(c))
            .last()
            .map_or(i, |(index, _)| index);
        let path = &text[path_start..i];
        let extension = path.rsplit_once('.').map(|(_, ext)| ext);
        let valid = line_end > i + 1
            && extension.is_some_and(|ext| {
                (1..=6).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric())
            })
            && !path.starts_with("//");
        if !valid {
            i += 1;
            continue;
        }
        let line: u32 = text[i + 1..line_end].parse().unwrap_or(0);
        let mut end = line_end;
        let mut column = 1;
        if text[end..].starts_with(':') {
            let column_end = digits(end + 1);
            if column_end > end + 1 {
                column = text[end + 1..column_end].parse().unwrap_or(1);
                end = column_end;
            }
        }
        let range = path_start..end;
        if line > 0 && !taken(&range, &links) {
            files.push(Link {
                range,
                target: LinkTarget::File {
                    path: path.to_owned(),
                    line,
                    column,
                },
            });
        }
        i = end.max(i + 1);
    }
    links.extend(files);
    links.sort_by_key(|link| link.range.start);
    links
}

impl IdeState {
    /// The project-relative file a location printed by a tool means: absolute paths inside the
    /// project, then relative to the task's working directory, then to the project root, then
    /// the one project file whose path ends with it (tools print module-relative paths).
    /// `None` when nothing in the project matches.
    pub fn resolve_console_path(&self, cwd: &Path, printed: &str) -> Option<PathBuf> {
        let project = self.project.as_ref()?;
        let root = project.root();
        let printed = Path::new(printed.trim_start_matches("./"));
        let inside = |absolute: PathBuf| {
            let relative = absolute.strip_prefix(root).ok()?.to_path_buf();
            root.join(&relative).is_file().then_some(relative)
        };
        if printed.is_absolute() {
            return inside(printed.to_path_buf());
        }
        let cwd = if cwd.is_absolute() {
            cwd.to_path_buf()
        } else {
            root.join(cwd)
        };
        if let Some(found) = inside(cwd.join(printed)).or_else(|| inside(root.join(printed))) {
            return Some(found);
        }
        let mut matches = self
            .files
            .iter()
            .filter(|entry| !entry.is_dir && entry.relative_path.ends_with(printed));
        let first = matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| first.relative_path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str) -> (PathBuf, IdeState) {
        let root =
            std::env::temp_dir().join(format!("archipel-links-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("mod/src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(root.join("mod/src/App.java"), "class App {}\n").unwrap();
        let mut state = IdeState::default();
        state.open_project(&root).unwrap();
        (root.canonicalize().unwrap(), state)
    }

    /// QA bug 7 of the TUI list: a relative location is found from the working directory, the project root
    /// or by its unique tail.
    #[test]
    fn relative_locations_resolve_against_cwd_root_and_project_files() {
        let (root, state) = project("resolve");
        let rel = |cwd: &Path, p: &str| state.resolve_console_path(cwd, p);
        assert_eq!(
            rel(&root, "src/main.rs"),
            Some(PathBuf::from("src/main.rs"))
        );
        assert_eq!(
            rel(&root.join("mod"), "src/App.java"),
            Some(PathBuf::from("mod/src/App.java"))
        );
        // Printed relative to the root while the task ran in a module.
        assert_eq!(
            rel(&root.join("mod"), "src/main.rs"),
            Some(PathBuf::from("src/main.rs"))
        );
        assert_eq!(
            rel(&root, "./src/main.rs"),
            Some(PathBuf::from("src/main.rs"))
        );
        // Only the tail is printed.
        assert_eq!(
            rel(&root, "App.java"),
            Some(PathBuf::from("mod/src/App.java"))
        );
        assert_eq!(
            rel(&root, &root.join("src/main.rs").display().to_string()),
            Some(PathBuf::from("src/main.rs"))
        );
        assert_eq!(rel(&root, "src/missing.rs"), None);
        assert_eq!(rel(&root, "/etc/passwd"), None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn urls_drop_trailing_punctuation() {
        let links = find("> Run with --info. See https://docs.gradle.org/8.5/userguide/x.html.");
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].target,
            LinkTarget::Url("https://docs.gradle.org/8.5/userguide/x.html".into())
        );
        let links = find("(see https://example.com/a_(b))");
        assert_eq!(
            links[0].target,
            LinkTarget::Url("https://example.com/a_(b)".into())
        );
    }

    #[test]
    fn locations_carry_line_and_column() {
        let text = "src/main/java/App.java:14: error: boom";
        let links = find(text);
        assert_eq!(links.len(), 1);
        assert_eq!(&text[links[0].range.clone()], "src/main/java/App.java:14");
        assert_eq!(
            links[0].target,
            LinkTarget::File {
                path: "src/main/java/App.java".into(),
                line: 14,
                column: 1
            }
        );
        let links = find("  --> src/lib.rs:7:12");
        assert_eq!(
            links[0].target,
            LinkTarget::File {
                path: "src/lib.rs".into(),
                line: 7,
                column: 12
            }
        );
    }

    #[test]
    fn ports_times_and_urls_are_not_files() {
        assert!(find("listening on localhost:8080").is_empty());
        assert!(find("took 12:30:45").is_empty());
        let links = find("https://example.com/a.html:80");
        assert_eq!(links.len(), 1);
        assert!(matches!(links[0].target, LinkTarget::Url(_)));
    }
}
