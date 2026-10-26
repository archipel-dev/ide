//! Simplified Android layout model for the layout editor: parses a layout XML
//! into a view tree and lays it out as labelled boxes. It is not the Android
//! layout engine: `LinearLayout` (orientation, `layout_weight`), `FrameLayout`
//! (stacked) and everything else (`ConstraintLayout`, `RelativeLayout`, …,
//! stacked top to bottom) are approximated.

use std::path::Path;

const ANDROID_NS: &str = "http://schemas.android.com/apk/res/android";

/// A view element of a layout file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    /// Element name without package, e.g. `LinearLayout`, `Button`.
    pub tag: String,
    /// `android:id` without the `@+id/` prefix.
    pub id: Option<String>,
    /// All attributes as `prefix:name` / `name` with their values.
    pub attributes: Vec<(String, String)>,
    /// 1-based line of the element.
    pub line: usize,
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    pub fn android_attr(&self, name: &str) -> Option<&str> {
        let key = format!("android:{name}");
        self.attributes
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn is_container(&self) -> bool {
        !self.children.is_empty()
            || self.tag.ends_with("Layout")
            || self.tag.ends_with("Group")
            || self.tag == "ScrollView"
    }

    /// `Button` plus its id, for tree rows and box labels.
    pub fn label(&self) -> String {
        match &self.id {
            Some(id) => format!("{} (@{id})", self.tag),
            None => self.tag.clone(),
        }
    }

    /// Pre-order index → node, counting `self` as 0.
    pub fn nth(&self, index: usize) -> Option<&LayoutNode> {
        fn walk<'a>(node: &'a LayoutNode, index: &mut usize) -> Option<&'a LayoutNode> {
            if *index == 0 {
                return Some(node);
            }
            *index -= 1;
            node.children.iter().find_map(|c| walk(c, index))
        }
        let mut index = index;
        walk(self, &mut index)
    }

    pub fn count(&self) -> usize {
        1 + self.children.iter().map(LayoutNode::count).sum::<usize>()
    }
}

/// Whether `path` (project-relative) is an Android layout resource file.
pub fn is_layout_file(path: &Path) -> bool {
    let parts: Vec<&str> = path.iter().filter_map(|p| p.to_str()).collect();
    let in_layout = parts
        .windows(2)
        .any(|w| w[0] == "res" && (w[1] == "layout" || w[1].starts_with("layout-")));
    in_layout && path.extension().is_some_and(|e| e == "xml")
}

/// Parses layout XML into its view tree.
pub fn parse_layout(xml: &str) -> Result<LayoutNode, String> {
    let document = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    Ok(node_of(xml, document.root_element()))
}

fn node_of(xml: &str, node: roxmltree::Node) -> LayoutNode {
    let tag = node.tag_name().name();
    let tag = tag.rsplit('.').next().unwrap_or(tag).to_owned();
    let attributes: Vec<(String, String)> = node
        .attributes()
        .map(|a| {
            let key = match a.namespace() {
                Some(ANDROID_NS) => format!("android:{}", a.name()),
                Some(ns) => node
                    .lookup_prefix(ns)
                    .map_or(a.name().to_owned(), |p| format!("{p}:{}", a.name())),
                None => a.name().to_owned(),
            };
            (key, a.value().to_owned())
        })
        .collect();
    let id = attributes
        .iter()
        .find(|(k, _)| k == "android:id")
        .map(|(_, v)| {
            v.trim_start_matches("@+id/")
                .trim_start_matches("@id/")
                .to_owned()
        });
    LayoutNode {
        tag,
        id,
        attributes,
        line: xml[..node.range().start].matches('\n').count() + 1,
        children: node
            .children()
            .filter(|c| c.is_element())
            .map(|c| node_of(xml, c))
            .collect(),
    }
}

/// A laid-out view.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedBox {
    /// Pre-order index of the node in the tree.
    pub index: usize,
    pub depth: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label: String,
}

const PADDING: f32 = 6.0;
const LEAF_HEIGHT: f32 = 36.0;

/// Lays the tree out inside a `width`×`height` screen. Boxes come out parents first.
pub fn layout_boxes(root: &LayoutNode, width: f32, height: f32) -> Vec<RenderedBox> {
    let mut out = Vec::new();
    let mut next = 0;
    place(root, (0.0, 0.0, width, height), 0, &mut next, &mut out);
    out
}

fn place(
    node: &LayoutNode,
    rect: (f32, f32, f32, f32),
    depth: usize,
    next: &mut usize,
    out: &mut Vec<RenderedBox>,
) {
    let index = *next;
    *next += 1;
    let (x, y, width, height) = rect;
    out.push(RenderedBox {
        index,
        depth,
        x,
        y,
        width,
        height,
        label: node.label(),
    });
    let count = node.children.len();
    if count == 0 {
        return;
    }
    let inner = (
        x + PADDING,
        y + PADDING + 14.0,
        (width - 2.0 * PADDING).max(0.0),
        (height - 2.0 * PADDING - 14.0).max(0.0),
    );
    let horizontal =
        node.tag == "LinearLayout" && node.android_attr("orientation") == Some("horizontal");
    let stacked = node.tag == "FrameLayout";
    let sizes = child_sizes(node, if horizontal { inner.2 } else { inner.3 });
    let mut cursor = 0.0;
    for (child, size) in node.children.iter().zip(sizes) {
        let child_rect = if stacked {
            // Overlapping, each inset a little so every box stays visible.
            let inset = (depth_of_sibling(node, child) as f32) * 10.0;
            (
                inner.0 + inset,
                inner.1 + inset,
                (inner.2 - inset * 1.5).max(0.0),
                (inner.3 - inset * 1.5).max(0.0),
            )
        } else if horizontal {
            let r = (inner.0 + cursor, inner.1, size, inner.3);
            cursor += size;
            r
        } else {
            let r = (inner.0, inner.1 + cursor, inner.2, size);
            cursor += size;
            r
        };
        place(child, child_rect, depth + 1, next, out);
    }
}

fn depth_of_sibling(parent: &LayoutNode, child: &LayoutNode) -> usize {
    parent
        .children
        .iter()
        .position(|c| std::ptr::eq(c, child))
        .unwrap_or(0)
}

/// Main-axis sizes of the children: weighted children share what the others
/// leave; the rest get a nominal size scaled to fit.
fn child_sizes(node: &LayoutNode, available: f32) -> Vec<f32> {
    let weights: Vec<f32> = node
        .children
        .iter()
        .map(|c| {
            c.android_attr("layout_weight")
                .and_then(|w| w.parse().ok())
                .unwrap_or(0.0)
        })
        .collect();
    let nominal: Vec<f32> = node
        .children
        .iter()
        .zip(&weights)
        .map(|(c, w)| {
            if *w > 0.0 {
                0.0
            } else if c.children.is_empty() {
                LEAF_HEIGHT
            } else {
                LEAF_HEIGHT * (1 + c.count()) as f32
            }
        })
        .collect();
    let fixed: f32 = nominal.iter().sum();
    let total_weight: f32 = weights.iter().sum();
    let scale = if fixed > available && fixed > 0.0 {
        available / fixed
    } else {
        1.0
    };
    let free = (available - fixed * scale).max(0.0);
    let unweighted = weights.iter().filter(|w| **w <= 0.0).count();
    nominal
        .iter()
        .zip(&weights)
        .map(|(n, w)| {
            if total_weight > 0.0 {
                if *w > 0.0 {
                    free * w / total_weight
                } else {
                    n * scale
                }
            } else if unweighted > 0 && fixed < available {
                // Match-parent style containers fill the space they get.
                n * scale + free / unweighted as f32
            } else {
                n * scale
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
    xmlns:app="http://schemas.android.com/apk/res-auto"
    android:layout_width="match_parent"
    android:orientation="vertical">
    <TextView android:id="@+id/title" android:text="Hi" />
    <androidx.constraintlayout.widget.ConstraintLayout android:layout_weight="1">
        <Button android:id="@+id/ok" app:layout_constraintTop_toTopOf="parent" />
    </androidx.constraintlayout.widget.ConstraintLayout>
</LinearLayout>"#;

    #[test]
    fn parses_tree_ids_and_namespaced_attributes() {
        let root = parse_layout(XML).unwrap();
        assert_eq!(root.tag, "LinearLayout");
        assert_eq!(root.count(), 4);
        assert_eq!(root.children[0].id.as_deref(), Some("title"));
        assert_eq!(root.children[1].tag, "ConstraintLayout");
        let button = root.nth(3).unwrap();
        assert_eq!(button.tag, "Button");
        assert_eq!(button.line, 8);
        assert!(
            button
                .attributes
                .iter()
                .any(|(k, _)| k == "app:layout_constraintTop_toTopOf")
        );
        assert_eq!(root.android_attr("orientation"), Some("vertical"));
        assert!(parse_layout("<a><b></a>").is_err());
    }

    #[test]
    fn lays_out_boxes_inside_their_parents() {
        let root = parse_layout(XML).unwrap();
        let boxes = layout_boxes(&root, 360.0, 640.0);
        assert_eq!(boxes.len(), 4);
        let (parent, title, constraint) = (&boxes[0], &boxes[1], &boxes[2]);
        for child in &boxes[1..] {
            assert!(child.x >= parent.x && child.x + child.width <= parent.x + parent.width + 0.01);
            assert!(child.y + child.height <= parent.y + parent.height + 0.01);
        }
        // The weighted constraint layout gets the space below the title.
        assert!(constraint.y >= title.y + title.height - 0.01);
        assert!(constraint.height > title.height);
        assert_eq!(boxes[3].depth, 2);
    }

    #[test]
    fn horizontal_linear_layout_splits_by_weight() {
        let xml = r#"<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
            android:orientation="horizontal">
            <View android:layout_weight="1"/><View android:layout_weight="3"/></LinearLayout>"#;
        let boxes = layout_boxes(&parse_layout(xml).unwrap(), 400.0, 100.0);
        assert!((boxes[2].width / boxes[1].width - 3.0).abs() < 0.01);
        assert!(boxes[2].x > boxes[1].x);
    }

    #[test]
    fn recognises_layout_files() {
        assert!(is_layout_file(Path::new(
            "app/src/main/res/layout/main.xml"
        )));
        assert!(is_layout_file(Path::new(
            "app/src/main/res/layout-land/main.xml"
        )));
        assert!(!is_layout_file(Path::new(
            "app/src/main/res/values/strings.xml"
        )));
        assert!(!is_layout_file(Path::new(
            "app/src/main/res/layout/main.png"
        )));
    }
}
