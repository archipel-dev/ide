//! Runner of `02-find-in-files.cases.md`: builds the in-memory project of the case, drives the
//! steps of the popup through [`FindRequest`] and compares rows, label, empty text, messages and
//! the files after a replacement.

use super::{Case, Outcome, compare_lines};
use archipel_ide_core::find_in_files::{
    FindRequest, ProjectFile, Scope, bracketed, empty_text, find, replace_all,
    replace_confirmation, validate,
};
use archipel_ide_search::SearchOptions;
use std::path::PathBuf;

struct Project {
    files: Vec<ProjectFile>,
    open: Vec<(PathBuf, String)>,
    modules: Vec<(String, PathBuf)>,
}

fn quoted(s: &str) -> Vec<String> {
    s.split('"').skip(1).step_by(2).map(str::to_owned).collect()
}

fn split_steps(keys: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = false;
    let chars: Vec<char> = keys.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '"' {
            quote = !quote;
        }
        if chars[i] == ',' && !quote && chars.get(i + 1) == Some(&' ') {
            out.push(std::mem::take(&mut cur).trim().to_owned());
            i += 2;
            continue;
        }
        cur.push(chars[i]);
        i += 1;
    }
    out.push(cur.trim().to_owned());
    out
}

fn parse_project(before: &str) -> Option<Project> {
    let mut files: Vec<ProjectFile> = Vec::new();
    let mut open_paths: Vec<String> = Vec::new();
    let mut modules = Vec::new();
    let mut excluded: Vec<String> = Vec::new();
    let mut cur: Option<(String, Vec<String>)> = None;
    let flush = |cur: &mut Option<(String, Vec<String>)>, files: &mut Vec<ProjectFile>| {
        if let Some((path, lines)) = cur.take() {
            let mut text = lines.join("\n");
            if !lines.is_empty() {
                text.push('\n');
            }
            files.push(ProjectFile::text(&path, &text.replace("<caret>", "")));
        }
    };
    for line in before.lines() {
        if let Some(path) = line.strip_prefix("--- ") {
            flush(&mut cur, &mut files);
            cur = Some((path.trim().to_owned(), Vec::new()));
        } else if let Some(p) = line.strip_prefix("open: ") {
            open_paths.push(p.trim().to_owned());
        } else if let Some(s) = line.strip_prefix("setup: ") {
            for part in s.split(';') {
                let part = part.trim();
                if let Some(rest) = part.strip_prefix("folder ") {
                    excluded.push(rest.split_whitespace().next()?.to_owned());
                } else if part.contains("belongs to a library root") {
                    excluded.push(part.split_whitespace().next()?.to_owned());
                } else if let Some(rest) = part.strip_prefix("modules ") {
                    for m in rest.split(" and ") {
                        let name = m.split_whitespace().next()?.to_owned();
                        let root = m
                            .split("content root ")
                            .nth(1)?
                            .trim_end_matches(')')
                            .trim();
                        modules.push((name, PathBuf::from(root)));
                    }
                }
            }
        } else if line.starts_with("focus:") || line.starts_with("modules:") {
            return None;
        } else if let Some((_, lines)) = &mut cur {
            lines.push(line.to_owned());
        }
    }
    flush(&mut cur, &mut files);
    for file in &mut files {
        if excluded.iter().any(|e| file.path.starts_with(e)) {
            file.excluded = true;
        }
        if file.path.to_string_lossy() == "b.png" {
            file.bytes = b"\x89PNG\0\0 needle \0".to_vec();
        }
    }
    let open = open_paths
        .iter()
        .filter_map(|p| {
            files
                .iter()
                .find(|f| f.path.as_path() == std::path::Path::new(p))
                .map(|f| {
                    (
                        f.path.clone(),
                        String::from_utf8_lossy(&f.bytes).into_owned(),
                    )
                })
        })
        .collect();
    Some(Project {
        files,
        open,
        modules,
    })
}

fn run_masks(case: &Case) -> Outcome {
    let names: Vec<&str> = case
        .before()
        .strip_prefix("names: ")
        .unwrap_or("")
        .split(", ")
        .collect();
    let mask = quoted(&case.keys).into_iter().next().unwrap_or_default();
    let options = SearchOptions {
        file_mask: mask,
        ..Default::default()
    };
    let Ok(matcher) = archipel_ide_search::Matcher::new("-", &options) else {
        return Outcome::Fail("mask does not compile".into());
    };
    let accepted: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| matcher.accepts(std::path::Path::new(n)))
        .collect();
    let want = case
        .after()
        .strip_prefix("accepted: ")
        .unwrap_or("")
        .split(", ")
        .collect::<Vec<_>>();
    if accepted == want {
        Outcome::Pass
    } else {
        Outcome::Fail(format!("accepted {accepted:?}, expected {want:?}"))
    }
}

pub fn run(case: &Case) -> Outcome {
    if case.keys.starts_with("mask \"") {
        return run_masks(case);
    }
    if case.after().contains("popup \"Find in Files\"") {
        return Outcome::NotRunnable("popup preset from the editor or project view context");
    }
    if case.after().contains("dimmed") {
        return Outcome::NotRunnable("row layout of the result list");
    }
    let popup_only = case.before().starts_with('(');
    let project = if popup_only {
        Project {
            files: Vec::new(),
            open: Vec::new(),
            modules: Vec::new(),
        }
    } else {
        let mut source = case.before().to_owned();
        for line in case.extra.iter().filter(|l| l.starts_with("setup:")) {
            source.push('\n');
            source.push_str(line);
        }
        match parse_project(&source) {
            Some(p) if !p.files.is_empty() => p,
            _ => {
                return Outcome::NotRunnable("needs the project view, modules or the editor state");
            }
        }
    };
    let mut request = FindRequest::default();
    let mut replace_mode = false;
    let mut replacement = String::new();
    let mut typing_replacement = false;
    let mut confirm = false;
    let mut confirmed = false;
    let mut find_window_title: Option<String> = None;
    let mut project = project;
    let st = case.settings.to_lowercase();
    request.options.match_case = st.contains("match case on");
    request.options.whole_words = st.contains("words on");
    request.options.regex = st.contains("regex on");
    let preserve = st.contains("preserve case on");
    for step in split_steps(&case.keys) {
        let s = step.as_str();
        if s == "Ctrl+Shift+F" {
        } else if s.contains("(unsaved)") {
            if let Some((_, text)) = project.open.first_mut() {
                let trimmed = text.trim_end_matches('\n').len();
                text.truncate(trimmed);
                text.push_str(&quoted(s).into_iter().next().unwrap_or_default());
            }
        } else if s == "Ctrl+Shift+R" {
            replace_mode = true;
        } else if let Some(rest) = s.strip_prefix("type \"") {
            let text = rest.trim_end_matches('"');
            if typing_replacement {
                replacement.push_str(text);
            } else {
                request.query.push_str(text);
            }
        } else if s == "Tab" {
            typing_replacement = true;
        } else if s == "Alt+X" {
            request.options.regex = !request.options.regex;
        } else if s == "Alt+C" {
            request.options.match_case = !request.options.match_case;
        } else if s.starts_with("choose scope Directory")
            || s.starts_with("scope Directory")
            || s.starts_with("directory \"")
        {
            let dir = quoted(s).into_iter().next().unwrap_or_default();
            let dir = dir.strip_prefix("<project>/").unwrap_or(&dir).to_owned();
            request.scope = Scope::Directory {
                path: dir.into(),
                recursive: true,
            };
        } else if s == "untick Recursively" {
            if let Scope::Directory { recursive, .. } = &mut request.scope {
                *recursive = false;
            }
        } else if s.starts_with("choose scope Module") {
            request.scope = Scope::Module {
                name: String::new(),
                root: PathBuf::new(),
            };
        } else if s.starts_with("module \"") {
            let name = quoted(s).into_iter().next().unwrap_or_default();
            let Some((_, root)) = project.modules.iter().find(|(n, _)| *n == name) else {
                return Outcome::NotRunnable("module scope needs the module model");
            };
            request.scope = Scope::Module {
                name,
                root: root.clone(),
            };
        } else if s.starts_with("mask \"")
            || s.starts_with("File mask \"")
            || s.starts_with("tick File mask")
        {
            request.file_mask_on = true;
            if let Some(m) = quoted(s).into_iter().next() {
                request.options.file_mask = m;
            }
        } else if s.starts_with("type \" // needle\"") {
            // Unsaved edit of the open file.
            if let Some((_, text)) = project.open.first_mut() {
                text.push_str(" // needle");
            }
        } else if s == "Alt+Shift+Enter" {
            confirm = true;
        } else if s == "click \"Replace\"" {
            confirmed = true;
        } else if s == "Ctrl+Enter" {
            let scope = match &request.scope {
                Scope::Project => "Project".to_owned(),
                Scope::Module { name, .. } => format!("Module '{name}'"),
                Scope::Directory { path, .. } => format!("Directory <project>/{}", path.display()),
            };
            let mask = if request.file_mask_on {
                format!(" with mask '{}'", request.options.file_mask)
            } else {
                String::new()
            };
            find_window_title = Some(format!(
                "Occurrences of '{}' in {scope}{mask}",
                request.query
            ));
        } else if s.starts_with("clear the file mask") {
            request.options.file_mask.clear();
        } else if s.contains("(unsaved)") || s.starts_with("then type") {
            return Outcome::NotRunnable("scripted editing steps");
        } else {
            return Outcome::NotRunnable("popup interaction that needs the window");
        }
    }
    let dir_exists = |p: &std::path::Path| project.files.iter().any(|f| f.path.starts_with(p));
    let after = case.after();
    let validation = validate(&request, dir_exists);
    // Validation messages.
    if after.starts_with("dialog: ") || after.starts_with("empty: \"Type search query") {
        let want = after.lines().next().unwrap_or("");
        let got = validation.clone().err();
        return match got {
            Some(msg)
                if want.contains(&msg) || want.contains(msg.split(':').next().unwrap_or("")) =>
            {
                Outcome::Pass
            }
            Some(msg) => Outcome::Fail(format!("message {msg:?}, expected {want}")),
            None if want.contains("no validation message") => Outcome::Pass,
            None => Outcome::NotRunnable("assertion about the look of the popup"),
        };
    }
    if let Err(msg) = &validation
        && !after.contains("Replace Error")
    {
        return Outcome::Fail(format!("unexpected validation message {msg}"));
    }
    let results = match find(&project.files, &project.open, &request) {
        Ok(r) => r,
        Err(e) => return Outcome::Fail(format!("{e}")),
    };
    // Expected sections.
    let mut want_rows: Vec<String> = Vec::new();
    let mut want_label: Option<String> = None;
    let mut want_empty: Vec<String> = Vec::new();
    let mut want_files: Vec<(String, Vec<String>)> = Vec::new();
    let mut mode = "";
    let mut extra: Vec<&str> = Vec::new();
    for line in after.lines() {
        if line == "list:" {
            mode = "list";
        } else if let Some(l) = line.strip_prefix("label: ") {
            want_label = Some(l.to_owned());
            mode = "";
        } else if line == "empty:" {
            mode = "empty";
        } else if let Some(p) = line.strip_prefix("--- ") {
            want_files.push((p.trim().to_owned(), Vec::new()));
            mode = "file";
        } else if line.starts_with("status:")
            || line.starts_with("dialog:")
            || line.starts_with("notification")
            || line.starts_with("local history")
            || line.starts_with("find tool window")
        {
            extra.push(line);
            mode = "";
        } else if mode == "list" && line.starts_with("selected row:") {
        } else if mode == "list" {
            want_rows.push(line.to_owned());
        } else if mode == "empty" {
            want_empty.push(line.to_owned());
        } else if mode == "file"
            && let Some((_, lines)) = want_files.last_mut()
        {
            if !line.starts_with('(') {
                lines.push(line.to_owned());
            }
        } else if !line.is_empty() && !line.starts_with('(') {
            extra.push(line);
        }
    }
    if !want_rows.is_empty() || after.starts_with("list:") {
        let got: Vec<String> = results
            .rows
            .iter()
            .map(|r| format!("{}:{}  {}", r.path.display(), r.line, bracketed(r)))
            .collect();
        if let Err(e) = compare_lines(&want_rows, &got) {
            return Outcome::Fail(e);
        }
    }
    if let Some(label) = want_label {
        let got = results.label();
        let want = label.split(" (").next().unwrap_or(&label).to_owned();
        if got != want {
            return Outcome::Fail(format!("label {got:?}, expected {want:?}"));
        }
    }
    if !want_empty.is_empty() {
        if !results.rows.is_empty() {
            return Outcome::Fail("rows found".into());
        }
        let got: Vec<String> = empty_text(&request).lines().map(str::to_owned).collect();
        let want: Vec<String> = want_empty
            .into_iter()
            .filter(|l| !l.starts_with('('))
            .collect();
        if let Err(e) = compare_lines(&want, &got) {
            return Outcome::Fail(e);
        }
    }
    if let Some(title) = find_window_title {
        let want = extra
            .iter()
            .find(|l| l.contains("title"))
            .copied()
            .unwrap_or("");
        if !want.contains(&title) && !after.contains(&title) {
            return Outcome::Fail(format!("title {title:?}"));
        }
    }
    if replace_mode || !want_files.is_empty() {
        let out = match replace_all(
            &project.files,
            &project.open,
            &request,
            &replacement,
            preserve,
        ) {
            Ok(o) => o,
            Err(e) => return Outcome::Fail(format!("{e}")),
        };
        if let Some(line) = extra
            .iter()
            .find(|l| l.starts_with("dialog:") && l.contains("Error"))
        {
            return match &out.error {
                Some(e) if line.contains(e.as_str()) => Outcome::Pass,
                other => Outcome::Fail(format!("error {other:?}")),
            };
        }
        if let Some(line) = extra.iter().find(|l| l.contains("Replace All\"")) {
            let (title, message) =
                replace_confirmation(&results, &request.query, &replacement).unwrap_or_default();
            if !line.contains(&title) || !line.contains(&message) {
                return Outcome::Fail(format!("confirmation {title}: {message}"));
            }
        }
        if confirm
            && !confirmed
            && replace_confirmation(&results, &request.query, &replacement).is_some()
        {
            return Outcome::Pass;
        }
        if let Some(line) = extra.iter().find(|l| l.starts_with("status:"))
            && !line.contains(&out.status)
        {
            return Outcome::Fail(format!("status {:?}", out.status));
        }
        for (path, lines) in &want_files {
            let new = out
                .changed
                .iter()
                .find(|(p, _)| p.to_string_lossy() == *path)
                .map(|(_, t)| t.clone())
                .or_else(|| {
                    project
                        .files
                        .iter()
                        .find(|f| f.path.to_string_lossy() == *path)
                        .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                })
                .unwrap_or_default();
            let want = lines
                .iter()
                .filter(|l| !l.starts_with("list:") && !l.starts_with("status"))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            if new.trim_end() != want.trim_end() {
                return Outcome::Fail(format!("{path}: {new:?} != {want:?}"));
            }
        }
        if let Some(line) = extra.iter().find(|l| l.starts_with("notification"))
            && out
                .notification
                .as_deref()
                .is_none_or(|n| !line.contains(n))
        {
            return Outcome::Fail(format!("notification {:?}", out.notification));
        }
        if let Some(line) = extra.iter().find(|l| l.starts_with("local history"))
            && !line.contains(&out.history_label)
        {
            return Outcome::Fail(format!("history label {:?}", out.history_label));
        }
    }
    Outcome::Pass
}
