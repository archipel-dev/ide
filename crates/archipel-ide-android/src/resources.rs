//! Android resource index and reference resolution: `R.string.app_name` in
//! Kotlin/Java and `@string/app_name` in XML resolve to the files and value
//! elements that declare them, across qualifier folders.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// A reference such as `R.layout.activity_main` or `@color/purple_500`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRef {
    pub kind: String,
    pub name: String,
}

/// Where a resource is declared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDef {
    pub kind: String,
    pub name: String,
    /// Project-relative file.
    pub path: PathBuf,
    /// 1-based line of the declaring element (1 for file resources).
    pub line: usize,
    /// Folder qualifier, e.g. `night` or `hdpi`; empty for the default folder.
    pub qualifier: String,
}

/// Folders whose files are resources named after the file.
const FILE_TYPES: [&str; 13] = [
    "layout",
    "drawable",
    "mipmap",
    "menu",
    "xml",
    "raw",
    "anim",
    "animator",
    "color",
    "font",
    "navigation",
    "transition",
    "interpolator",
];

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

/// Finds the resource reference under `offset` (a char index) in `line`.
/// Framework references (`android.R.…`, `@android:…`) are ignored.
pub fn reference_at(line: &str, offset: usize) -> Option<ResourceRef> {
    let chars: Vec<char> = line.chars().collect();
    let offset = offset.min(chars.len());
    // Expand to the token around the caret: name chars plus '@', '/', '+', ':'.
    let in_token = |c: char| is_name_char(c) || matches!(c, '@' | '/' | '+' | ':');
    let mut start = offset;
    while start > 0 && in_token(chars[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < chars.len() && in_token(chars[end]) {
        end += 1;
    }
    let token: String = chars[start..end].iter().collect();
    if let Some(rest) = token.find('@').map(|i| &token[i + 1..]) {
        let rest = rest.trim_start_matches('+');
        if rest.contains(':') {
            return None;
        }
        let (kind, name) = rest.split_once('/')?;
        // `@style/Theme.App` is `R.style.Theme_App`.
        return valid(kind, &name.replace('.', "_"));
    }
    // `R.type.name`, possibly qualified (`com.example.R.string.x`) or followed by more.
    let parts: Vec<&str> = token.split('.').collect();
    let r = parts.iter().position(|p| *p == "R")?;
    if r > 0 && parts[r - 1] == "android" {
        return None;
    }
    valid(parts.get(r + 1)?, parts.get(r + 2)?)
}

fn valid(kind: &str, name: &str) -> Option<ResourceRef> {
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    (ok(kind) && ok(name)).then(|| ResourceRef {
        kind: kind.to_owned(),
        name: name.to_owned(),
    })
}

/// Splits `res/<folder>/<file>` of a project-relative path.
fn res_parts(path: &Path) -> Option<(&str, &str)> {
    let mut components = path.components().rev();
    let file = components.next()?.as_os_str().to_str()?;
    let folder = components.next()?.as_os_str().to_str()?;
    let res = components.next()?.as_os_str().to_str()?;
    (res == "res").then_some((folder, file))
}

/// Index of every resource declared under the given files.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ResourceIndex {
    pub defs: Vec<ResourceDef>,
}

impl ResourceIndex {
    /// Builds the index from project-relative `files` under `root`. Value files
    /// and layouts are read to find value elements and `@+id/` declarations.
    pub fn build(root: &Path, files: &[PathBuf]) -> Self {
        let mut defs = Vec::new();
        for path in files {
            let Some((folder, file)) = res_parts(path) else {
                continue;
            };
            let (kind, qualifier) = folder.split_once('-').unwrap_or((folder, ""));
            if FILE_TYPES.contains(&kind) {
                let name = file.split('.').next().unwrap_or(file);
                defs.push(ResourceDef {
                    kind: kind.to_owned(),
                    name: name.to_owned(),
                    path: path.clone(),
                    line: 1,
                    qualifier: qualifier.to_owned(),
                });
            }
            let xml = file.ends_with(".xml");
            if xml
                && matches!(kind, "values" | "layout" | "menu" | "navigation")
                && let Ok(text) = std::fs::read_to_string(root.join(path))
            {
                defs.extend(xml_defs(&text, path, qualifier, kind == "values"));
            }
        }
        Self { defs }
    }

    /// Declarations of `reference`, default-qualifier folders first.
    pub fn lookup(&self, reference: &ResourceRef) -> Vec<&ResourceDef> {
        let mut found: Vec<&ResourceDef> = self
            .defs
            .iter()
            .filter(|d| d.kind == reference.kind && d.name == reference.name)
            .collect();
        found.sort_by(|a, b| {
            (!a.qualifier.is_empty(), &a.qualifier, &a.path, a.line).cmp(&(
                !b.qualifier.is_empty(),
                &b.qualifier,
                &b.path,
                b.line,
            ))
        });
        found
    }
}

fn line_of(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].matches('\n').count() + 1
}

/// Value elements (`<string name=…>`, `<item type="id" …>`, `<declare-styleable>`)
/// and `@+id/` declarations.
fn xml_defs(text: &str, path: &Path, qualifier: &str, values: bool) -> Vec<ResourceDef> {
    let Ok(document) = roxmltree::Document::parse(text) else {
        return Vec::new();
    };
    let def = |kind: &str, name: &str, node: roxmltree::Node| ResourceDef {
        kind: kind.to_owned(),
        name: name.replace('.', "_"),
        path: path.to_owned(),
        line: line_of(text, node.range().start),
        qualifier: qualifier.to_owned(),
    };
    let mut defs = Vec::new();
    for node in document.descendants().filter(|n| n.is_element()) {
        if values && node.parent().is_some_and(|p| p.has_tag_name("resources")) {
            let Some(name) = node.attribute("name") else {
                continue;
            };
            let kind = match node.tag_name().name() {
                "item" => node.attribute("type").unwrap_or("item"),
                "string-array" | "integer-array" | "array" => "array",
                "declare-styleable" => "styleable",
                "eat-comment" | "skip" => continue,
                tag => tag,
            };
            defs.push(def(kind, name, node));
        }
        for attribute in node.attributes() {
            if let Some(id) = attribute.value().strip_prefix("@+id/") {
                defs.push(def("id", id, node));
            }
        }
    }
    defs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_references_under_the_caret() {
        let line = "        setContentView(R.layout.activity_main)";
        let at = line.find("activity").unwrap();
        assert_eq!(
            reference_at(line, at),
            Some(ResourceRef {
                kind: "layout".into(),
                name: "activity_main".into()
            })
        );
        let at = line.find("R.layout").unwrap();
        assert_eq!(reference_at(line, at).unwrap().kind, "layout");
        let xml = r#"    <application android:label="@string/app_name" android:theme="@style/Theme.App">"#;
        assert_eq!(
            reference_at(xml, xml.find("app_name").unwrap())
                .unwrap()
                .name,
            "app_name"
        );
        assert_eq!(
            reference_at(xml, xml.find("Theme").unwrap()).unwrap().name,
            "Theme_App"
        );
        assert_eq!(
            reference_at(r#"android:id="@+id/greeting""#, 20)
                .unwrap()
                .kind,
            "id"
        );
        assert_eq!(reference_at("android.R.string.ok", 12), None);
        assert_eq!(reference_at("@android:color/white", 10), None);
        assert_eq!(reference_at("val x = 1", 4), None);
        assert_eq!(reference_at("", 5), None);
        let qualified = "com.example.app.R.string.title";
        assert_eq!(reference_at(qualified, 28).unwrap().name, "title");
    }

    #[test]
    fn indexes_values_files_and_ids() {
        let dir = crate::adb::tests::temp("resources");
        let write = |rel: &str, text: &str| {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write(
            "app/src/main/res/values/strings.xml",
            "<resources>\n    <string name=\"app_name\">My App</string>\n    <string-array name=\"planets\"><item>a</item></string-array>\n    <item type=\"id\" name=\"spare\"/>\n    <style name=\"Theme.App\" parent=\"x\"/>\n</resources>\n",
        );
        write(
            "app/src/main/res/values-night/strings.xml",
            "<resources>\n\n  <string name=\"app_name\">My App (dark)</string>\n</resources>\n",
        );
        write(
            "app/src/main/res/layout/activity_main.xml",
            "<LinearLayout xmlns:android=\"http://schemas.android.com/apk/res/android\">\n  <TextView\n    android:id=\"@+id/greeting\"/>\n</LinearLayout>\n",
        );
        write("app/src/main/res/mipmap-hdpi/ic_launcher.webp", "");
        write("app/src/main/res/drawable/frame.9.png", "");
        write(
            "app/src/main/res/values/broken.xml",
            "<resources><string name=",
        );
        let files: Vec<PathBuf> = [
            "app/src/main/res/values/strings.xml",
            "app/src/main/res/values-night/strings.xml",
            "app/src/main/res/layout/activity_main.xml",
            "app/src/main/res/mipmap-hdpi/ic_launcher.webp",
            "app/src/main/res/drawable/frame.9.png",
            "app/src/main/res/values/broken.xml",
            "app/src/main/java/a/B.kt",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        let index = ResourceIndex::build(&dir, &files);
        let r = |kind: &str, name: &str| ResourceRef {
            kind: kind.into(),
            name: name.into(),
        };
        let app_name = index.lookup(&r("string", "app_name"));
        assert_eq!(app_name.len(), 2);
        assert_eq!((app_name[0].line, app_name[0].qualifier.as_str()), (2, ""));
        assert_eq!(
            (app_name[1].line, app_name[1].qualifier.as_str()),
            (3, "night")
        );
        assert_eq!(index.lookup(&r("array", "planets"))[0].line, 3);
        assert_eq!(index.lookup(&r("id", "spare"))[0].line, 4);
        assert_eq!(index.lookup(&r("style", "Theme_App"))[0].line, 5);
        let greeting = index.lookup(&r("id", "greeting"));
        assert_eq!(
            greeting[0].path,
            PathBuf::from("app/src/main/res/layout/activity_main.xml")
        );
        assert_eq!(greeting[0].line, 2);
        assert_eq!(index.lookup(&r("layout", "activity_main"))[0].line, 1);
        assert_eq!(
            index.lookup(&r("mipmap", "ic_launcher"))[0].qualifier,
            "hdpi"
        );
        assert_eq!(index.lookup(&r("drawable", "frame")).len(), 1);
        assert!(index.lookup(&r("string", "missing")).is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
