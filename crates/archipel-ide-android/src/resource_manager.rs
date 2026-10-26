//! Resource Manager catalog: groups the resource index by type and name for
//! the tool window's tabs, with the value or file used to preview each entry.

use crate::resources::{ResourceDef, ResourceIndex};
use crate::vector::parse_color;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Tabs of the Resource Manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize)]
pub enum ResourceTab {
    #[default]
    Drawable,
    Color,
    Layout,
    MipMap,
    String,
}

impl ResourceTab {
    pub const ALL: [ResourceTab; 5] = [
        ResourceTab::Drawable,
        ResourceTab::Color,
        ResourceTab::Layout,
        ResourceTab::MipMap,
        ResourceTab::String,
    ];

    pub fn title(self) -> &'static str {
        match self {
            ResourceTab::Drawable => "Drawable",
            ResourceTab::Color => "Color",
            ResourceTab::Layout => "Layout",
            ResourceTab::MipMap => "Mip Map",
            ResourceTab::String => "String",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            ResourceTab::Drawable => "drawable",
            ResourceTab::Color => "color",
            ResourceTab::Layout => "layout",
            ResourceTab::MipMap => "mipmap",
            ResourceTab::String => "string",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.id() == id)
    }
}

/// What a grid cell shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Preview {
    /// Raster image file (project-relative).
    Image(PathBuf),
    /// `<vector>` drawable file (project-relative).
    Vector(PathBuf),
    /// ARGB colour.
    Color(u32),
    /// Text value, or a literal description.
    Text(String),
    None,
}

/// One resource name with all its qualifier versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceItem {
    pub tab: ResourceTab,
    pub name: String,
    /// Module directory relative to the project root.
    pub module: PathBuf,
    /// Number of declarations (qualifier folders / value files).
    pub versions: usize,
    /// First declaration, default qualifier first.
    pub path: PathBuf,
    pub line: usize,
    pub preview: Preview,
}

fn tab_of(kind: &str) -> Option<ResourceTab> {
    Some(match kind {
        "drawable" => ResourceTab::Drawable,
        "color" => ResourceTab::Color,
        "layout" => ResourceTab::Layout,
        "mipmap" => ResourceTab::MipMap,
        "string" => ResourceTab::String,
        _ => return None,
    })
}

/// Value text of the `<kind name=…>` element at `def`, read from its values file.
fn value_of(root: &Path, def: &ResourceDef) -> Option<String> {
    let text = std::fs::read_to_string(root.join(&def.path)).ok()?;
    let document = roxmltree::Document::parse(&text).ok()?;
    document
        .descendants()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == def.kind
                && n.attribute("name")
                    .map(|name| name.replace('.', "_"))
                    .as_deref()
                    == Some(def.name.as_str())
        })
        .map(|n| n.text().unwrap_or_default().trim().to_owned())
}

fn preview(root: &Path, def: &ResourceDef) -> Preview {
    let extension = def
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match (def.kind.as_str(), extension.as_str()) {
        ("drawable" | "mipmap", "png" | "jpg" | "jpeg" | "webp" | "gif") => {
            Preview::Image(def.path.clone())
        }
        ("drawable" | "mipmap", "xml") => {
            let is_vector = std::fs::read_to_string(root.join(&def.path))
                .map(|text| text.contains("<vector"))
                .unwrap_or(false);
            if is_vector {
                Preview::Vector(def.path.clone())
            } else {
                Preview::None
            }
        }
        ("color", _) if def.path.components().any(|c| c.as_os_str() == "values") => {
            match value_of(root, def) {
                Some(value) => parse_color(&value).map_or(Preview::Text(value), Preview::Color),
                None => Preview::None,
            }
        }
        ("string", _) => value_of(root, def).map_or(Preview::None, Preview::Text),
        _ => Preview::None,
    }
}

/// Builds the catalog for `modules` (directories relative to `root`); a
/// resource belongs to the module whose directory is the longest prefix of its path.
pub fn catalog(root: &Path, files: &[PathBuf], modules: &[PathBuf]) -> Vec<ResourceItem> {
    let index = ResourceIndex::build(root, files);
    let mut groups: BTreeMap<(PathBuf, ResourceTab, String), Vec<&ResourceDef>> = BTreeMap::new();
    for def in &index.defs {
        let Some(tab) = tab_of(&def.kind) else {
            continue;
        };
        let Some(module) = modules
            .iter()
            .filter(|m| def.path.starts_with(m))
            .max_by_key(|m| m.components().count())
        else {
            continue;
        };
        groups
            .entry((module.clone(), tab, def.name.clone()))
            .or_default()
            .push(def);
    }
    groups
        .into_iter()
        .map(|((module, tab, name), mut defs)| {
            defs.sort_by(|a, b| {
                (!a.qualifier.is_empty(), &a.qualifier, &a.path, a.line).cmp(&(
                    !b.qualifier.is_empty(),
                    &b.qualifier,
                    &b.path,
                    b.line,
                ))
            });
            let first = defs[0];
            ResourceItem {
                tab,
                name,
                module,
                versions: defs.len(),
                path: first.path.clone(),
                line: first.line,
                preview: preview(root, first),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) -> PathBuf {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        PathBuf::from(rel)
    }

    #[test]
    fn groups_by_tab_with_previews_and_versions() {
        let root = std::env::temp_dir().join(format!("archipel-ide-resmgr-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let files = vec![
            write(&root, "app/src/main/res/drawable/icon.png", "x"),
            write(
                &root,
                "app/src/main/res/drawable/arrow.xml",
                "<vector xmlns:android=\"http://schemas.android.com/apk/res/android\"/>",
            ),
            write(
                &root,
                "app/src/main/res/drawable/selector.xml",
                "<selector/>",
            ),
            write(&root, "app/src/main/res/layout/main.xml", "<LinearLayout/>"),
            write(
                &root,
                "app/src/main/res/layout-land/main.xml",
                "<LinearLayout/>",
            ),
            write(&root, "app/src/main/res/mipmap-hdpi/ic_launcher.png", "x"),
            write(
                &root,
                "app/src/main/res/values/values.xml",
                "<resources><color name=\"purple\">#6200EE</color><string name=\"app_name\">My App</string></resources>",
            ),
            write(&root, "lib/src/main/res/drawable/lib_icon.png", "x"),
        ];
        let modules = [PathBuf::from("app"), PathBuf::from("lib")];
        let items = catalog(&root, &files, &modules);
        let find = |tab, name: &str| {
            items
                .iter()
                .find(|i| i.tab == tab && i.name == name)
                .unwrap_or_else(|| panic!("missing {name}"))
        };
        assert!(matches!(
            find(ResourceTab::Drawable, "icon").preview,
            Preview::Image(_)
        ));
        assert!(matches!(
            find(ResourceTab::Drawable, "arrow").preview,
            Preview::Vector(_)
        ));
        assert_eq!(
            find(ResourceTab::Drawable, "selector").preview,
            Preview::None
        );
        assert_eq!(find(ResourceTab::Layout, "main").versions, 2);
        assert_eq!(
            find(ResourceTab::Color, "purple").preview,
            Preview::Color(0xFF6200EE)
        );
        assert_eq!(
            find(ResourceTab::String, "app_name").preview,
            Preview::Text("My App".into())
        );
        assert_eq!(find(ResourceTab::MipMap, "ic_launcher").versions, 1);
        assert_eq!(
            find(ResourceTab::Drawable, "lib_icon").module,
            PathBuf::from("lib")
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
