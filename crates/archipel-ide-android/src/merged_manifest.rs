//! Merged manifest view: the manifest AGP produced for a variant, with the
//! source of every line taken from the merger's blame report
//! (`manifest-merger-blame-<variant>-report.txt`).

use serde::Serialize;
use std::path::{Path, PathBuf};

/// Where a merged line came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ManifestSource {
    /// A manifest in the project (absolute path, 1-based line).
    File { path: PathBuf, line: usize },
    /// A library manifest, e.g. `androidx.core:core:1.13.0`.
    Library {
        coordinate: String,
        path: PathBuf,
        line: usize,
    },
    /// Not in any manifest: injected from the Gradle build (versionCode,
    /// uses-sdk, debuggable, …).
    Injected,
}

impl ManifestSource {
    /// Legend label, e.g. `app main manifest`, `core:1.13.0 manifest`.
    pub fn label(&self, project_root: &Path) -> String {
        match self {
            ManifestSource::File { path, .. } => {
                let relative = path.strip_prefix(project_root).unwrap_or(path);
                format!("{} manifest", relative.display())
            }
            ManifestSource::Library { coordinate, .. } => {
                let short = coordinate
                    .split_once(':')
                    .map(|(_, r)| r)
                    .unwrap_or(coordinate);
                format!("{short} manifest")
            }
            ManifestSource::Injected => "build.gradle injection".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergedLine {
    pub text: String,
    /// Index into [`MergedManifest::sources`].
    pub source: usize,
    /// 1-based line in the source manifest, when attributed.
    pub source_line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergedManifest {
    pub blame_file: PathBuf,
    pub lines: Vec<MergedLine>,
    /// Distinct sources in order of first use; project files first.
    pub sources: Vec<ManifestSource>,
}

fn parse_location(text: &str) -> Option<(PathBuf, usize)> {
    // `/abs/AndroidManifest.xml:3:5-10:19` or `…:6:25-66`; the path may contain ':'.
    static LOCATION: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let pattern = LOCATION.get_or_init(|| {
        regex::Regex::new(r"^(.+):(\d+):\d+(?:-(?:\d+:)?\d+)?$").expect("valid regex")
    });
    let captures = pattern.captures(text.trim())?;
    Some((PathBuf::from(&captures[1]), captures[2].parse().ok()?))
}

fn parse_source(text: &str) -> ManifestSource {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('[')
        && let Some((coordinate, location)) = rest.split_once(']')
    {
        let (path, line) = parse_location(location).unwrap_or_default();
        return ManifestSource::Library {
            coordinate: coordinate.trim().to_owned(),
            path,
            line,
        };
    }
    match parse_location(text) {
        Some((path, line)) => ManifestSource::File { path, line },
        None => ManifestSource::Injected,
    }
}

impl MergedManifest {
    /// Parses a blame report: `N<text>` lines are the merged manifest, and
    /// `N--><source>` lines attribute line N. Lines without attribution are
    /// injected by the build.
    pub fn parse(report: &str, blame_file: &Path) -> Self {
        let mut lines: Vec<(String, Option<ManifestSource>)> = Vec::new();
        for raw in report.lines() {
            let digits = raw.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits == 0 {
                continue;
            }
            let Ok(number) = raw[..digits].parse::<usize>() else {
                continue;
            };
            let rest = &raw[digits..];
            if let Some(source) = rest.strip_prefix("-->") {
                if let Some((_, existing)) = lines.get_mut(number.saturating_sub(1))
                    && existing.is_none()
                {
                    *existing = Some(parse_source(source));
                }
            } else if number == lines.len() + 1 {
                lines.push((rest.to_owned(), None));
            }
        }
        let mut sources: Vec<ManifestSource> = Vec::new();
        let mut index_of = |source: ManifestSource| match sources.iter().position(|s| *s == source)
        {
            Some(i) => i,
            None => {
                sources.push(source);
                sources.len() - 1
            }
        };
        // One legend entry per file/library, not per line.
        let key = |source: &ManifestSource| match source {
            ManifestSource::File { path, .. } => ManifestSource::File {
                path: path.clone(),
                line: 0,
            },
            ManifestSource::Library {
                coordinate, path, ..
            } => ManifestSource::Library {
                coordinate: coordinate.clone(),
                path: path.clone(),
                line: 0,
            },
            ManifestSource::Injected => ManifestSource::Injected,
        };
        let mut previous = None;
        let lines = lines
            .into_iter()
            .map(|(text, source)| {
                let trimmed = text.trim();
                let inherit = trimmed.is_empty() || trimmed.starts_with("</");
                let source_line = match &source {
                    Some(ManifestSource::File { line, .. })
                    | Some(ManifestSource::Library { line, .. }) => Some(*line),
                    _ => None,
                };
                let source = match (source, inherit, previous) {
                    (Some(source), _, _) => index_of(key(&source)),
                    (None, true, Some(previous)) => previous,
                    (None, _, _) => index_of(ManifestSource::Injected),
                };
                previous = Some(source);
                MergedLine {
                    source,
                    source_line,
                    text,
                }
            })
            .collect();
        Self {
            blame_file: blame_file.to_owned(),
            lines,
            sources,
        }
    }

    /// Finds and parses the blame report of `variant` under `module_dir`.
    pub fn load(module_dir: &Path, variant: &str) -> Result<Self, String> {
        let dir = module_dir
            .join("build/intermediates/manifest_merge_blame_file")
            .join(variant);
        let file_name = format!("manifest-merger-blame-{variant}-report.txt");
        let found = walkdir::WalkDir::new(&dir)
            .max_depth(2)
            .into_iter()
            .flatten()
            .find(|e| e.file_name().to_str() == Some(file_name.as_str()))
            .map(|e| e.into_path())
            .ok_or_else(|| {
                format!(
                    "No merged manifest for {variant} yet: build the app (assemble{}) to produce {}.",
                    {
                        let mut c = variant.to_owned();
                        if let Some(first) = c.get_mut(..1) {
                            first.make_ascii_uppercase();
                        }
                        c
                    },
                    dir.join(&file_name).display()
                )
            })?;
        let text = std::fs::read_to_string(&found)
            .map_err(|e| format!("cannot read {}: {e}", found.display()))?;
        Ok(Self::parse(&text, &found))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = r#"1<?xml version="1.0" encoding="utf-8"?>
2<manifest xmlns:android="http://schemas.android.com/apk/res/android"
3    package="com.example.hello"
4    android:versionCode="1" >
5
6    <permission
6-->[androidx.core:core:1.13.0] /home/me/.gradle/caches/transformed/core-1.13.0/AndroidManifest.xml:22:5-24:47
7        android:name="com.example.hello.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION" />
7-->[androidx.core:core:1.13.0] /home/me/.gradle/caches/transformed/core-1.13.0/AndroidManifest.xml:23:9-81
8    <application
8-->/work/HelloAndroid/app/src/main/AndroidManifest.xml:3:5-10:19
9        android:label="@string/app_name" >
9-->/work/HelloAndroid/app/src/main/AndroidManifest.xml:3:18-50
9-->/work/other/AndroidManifest.xml:1:1-2
10    </application>
11
12</manifest>
"#;

    #[test]
    fn parses_blame_report() {
        let merged = MergedManifest::parse(REPORT, Path::new("blame.txt"));
        assert_eq!(merged.lines.len(), 12);
        assert_eq!(merged.lines[2].text, "    package=\"com.example.hello\"");
        assert_eq!(merged.sources.len(), 3);
        assert_eq!(
            merged.sources[merged.lines[0].source],
            ManifestSource::Injected
        );
        let library = &merged.sources[merged.lines[5].source];
        assert_eq!(
            library.label(Path::new("/work/HelloAndroid")),
            "core:1.13.0 manifest"
        );
        let main = &merged.sources[merged.lines[8].source];
        assert_eq!(
            main.label(Path::new("/work/HelloAndroid")),
            "app/src/main/AndroidManifest.xml manifest"
        );
        assert_eq!(
            merged.lines[7].source, merged.lines[8].source,
            "first attribution wins"
        );
        assert_eq!(merged.lines[6].source, merged.lines[5].source);
        assert_eq!(
            merged.lines[9].source, merged.lines[8].source,
            "closing tags inherit the previous line's source"
        );
        assert_eq!(merged.lines[11].source, merged.lines[8].source);
        assert_eq!(merged.lines[8].source_line, Some(3));
        assert_eq!(merged.lines[0].source_line, None);
        assert!(
            MergedManifest::parse("garbage\n-->x\n", Path::new("b"))
                .lines
                .is_empty()
        );
    }

    #[test]
    fn missing_report_explains_how_to_produce_it() {
        let error = MergedManifest::load(Path::new("/nonexistent/app"), "freeDebug").unwrap_err();
        assert!(error.contains("assembleFreeDebug"), "{error}");
    }
}
