//! Editor split model: a binary tree of tab groups (PLATFORM-UI-004).
//!
//! UI independent: a group holds ordered tab keys and remembers its own active
//! tab; the focused group is the one commands act on. The model is generic over
//! the tab key (the IDE uses project-relative paths) and every tab lives in at
//! most one group.

use serde::{Deserialize, Serialize};

pub type GroupId = u32;

/// How the children of a split are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    /// Side by side (Split Right).
    Columns,
    /// Stacked (Split Down).
    Rows,
}

/// Which side of a group a new split is created on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

impl Edge {
    fn orientation(self) -> Orientation {
        match self {
            Edge::Left | Edge::Right => Orientation::Columns,
            Edge::Top | Edge::Bottom => Orientation::Rows,
        }
    }

    fn new_group_first(self) -> bool {
        matches!(self, Edge::Left | Edge::Top)
    }
}

/// A screen rectangle in any unit; the model never draws.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

/// The edge of `rect` the point is within `fraction` of (0.0..0.5), if any; the
/// nearest edge wins in corners. Used to turn a tab drop into a split.
pub fn drop_edge(rect: Rect, x: f32, y: f32, fraction: f32) -> Option<Edge> {
    if !rect.contains(x, y) || rect.w <= 0.0 || rect.h <= 0.0 {
        return None;
    }
    let (fx, fy) = ((x - rect.x) / rect.w, (y - rect.y) / rect.h);
    let candidates = [
        (fx, Edge::Left),
        (1.0 - fx, Edge::Right),
        (fy, Edge::Top),
        (1.0 - fy, Edge::Bottom),
    ];
    candidates
        .into_iter()
        .filter(|(distance, _)| *distance <= fraction)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, edge)| edge)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group<T> {
    pub id: GroupId,
    pub tabs: Vec<T>,
    /// The tab this group shows; remembered while another group has focus.
    pub active: Option<T>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Node<T> {
    Leaf(Group<T>),
    Split {
        orientation: Orientation,
        /// Share of the first child, 0.1..=0.9.
        ratio: f32,
        first: Box<Node<T>>,
        second: Box<Node<T>>,
    },
}

/// A draggable divider between two children.
#[derive(Debug, Clone, PartialEq)]
pub struct Divider {
    /// Child choices from the root to the split (false = first).
    pub path: Vec<bool>,
    pub orientation: Orientation,
    /// The area of the whole split the ratio refers to.
    pub area: Rect,
    pub ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitTree<T> {
    root: Node<T>,
    focused: GroupId,
    next_id: GroupId,
}

impl<T> Default for SplitTree<T> {
    fn default() -> Self {
        Self {
            root: Node::Leaf(Group {
                id: 0,
                tabs: Vec::new(),
                active: None,
            }),
            focused: 0,
            next_id: 1,
        }
    }
}

impl<T> Node<T> {
    fn leaves<'a>(&'a self, out: &mut Vec<&'a Group<T>>) {
        match self {
            Node::Leaf(group) => out.push(group),
            Node::Split { first, second, .. } => {
                first.leaves(out);
                second.leaves(out);
            }
        }
    }

    fn leaf_mut(&mut self, id: GroupId) -> Option<&mut Group<T>> {
        match self {
            Node::Leaf(group) => (group.id == id).then_some(group),
            Node::Split { first, second, .. } => first.leaf_mut(id).or_else(|| second.leaf_mut(id)),
        }
    }

    fn leaves_mut<'a>(&'a mut self, out: &mut Vec<&'a mut Group<T>>) {
        match self {
            Node::Leaf(group) => out.push(group),
            Node::Split { first, second, .. } => {
                first.leaves_mut(out);
                second.leaves_mut(out);
            }
        }
    }

    /// Removes the leaf `id`, replacing its parent split by the sibling.
    fn remove_leaf(&mut self, id: GroupId) -> bool {
        let Node::Split { first, second, .. } = self else {
            return false;
        };
        let is_leaf = |node: &Node<T>| matches!(node, Node::Leaf(g) if g.id == id);
        let replacement = if is_leaf(first) {
            Some(std::mem::replace(&mut **second, placeholder()))
        } else if is_leaf(second) {
            Some(std::mem::replace(&mut **first, placeholder()))
        } else {
            None
        };
        match replacement {
            Some(node) => {
                *self = node;
                true
            }
            None => first.remove_leaf(id) || second.remove_leaf(id),
        }
    }

    fn split_leaf(&mut self, id: GroupId, edge: Edge, new_group: Group<T>) -> Result<(), Group<T>> {
        match self {
            Node::Leaf(group) if group.id == id => {
                let old = std::mem::replace(self, placeholder());
                let new = Node::Leaf(new_group);
                let (first, second) = if edge.new_group_first() {
                    (new, old)
                } else {
                    (old, new)
                };
                *self = Node::Split {
                    orientation: edge.orientation(),
                    ratio: 0.5,
                    first: Box::new(first),
                    second: Box::new(second),
                };
                Ok(())
            }
            Node::Leaf(_) => Err(new_group),
            Node::Split { first, second, .. } => first
                .split_leaf(id, edge, new_group)
                .or_else(|group| second.split_leaf(id, edge, group)),
        }
    }

    fn layout(&self, rect: Rect, out: &mut Vec<(GroupId, Rect)>) {
        match self {
            Node::Leaf(group) => out.push((group.id, rect)),
            Node::Split {
                orientation,
                ratio,
                first,
                second,
            } => {
                let (a, b) = divide(rect, *orientation, *ratio);
                first.layout(a, out);
                second.layout(b, out);
            }
        }
    }

    fn dividers(&self, rect: Rect, path: &mut Vec<bool>, out: &mut Vec<Divider>) {
        if let Node::Split {
            orientation,
            ratio,
            first,
            second,
        } = self
        {
            out.push(Divider {
                path: path.clone(),
                orientation: *orientation,
                area: rect,
                ratio: *ratio,
            });
            let (a, b) = divide(rect, *orientation, *ratio);
            path.push(false);
            first.dividers(a, path, out);
            path.pop();
            path.push(true);
            second.dividers(b, path, out);
            path.pop();
        }
    }
}

fn placeholder<T>() -> Node<T> {
    Node::Leaf(Group {
        id: GroupId::MAX,
        tabs: Vec::new(),
        active: None,
    })
}

fn divide(rect: Rect, orientation: Orientation, ratio: f32) -> (Rect, Rect) {
    match orientation {
        Orientation::Columns => {
            let w = (rect.w * ratio).round();
            (
                Rect::new(rect.x, rect.y, w, rect.h),
                Rect::new(rect.x + w, rect.y, rect.w - w, rect.h),
            )
        }
        Orientation::Rows => {
            let h = (rect.h * ratio).round();
            (
                Rect::new(rect.x, rect.y, rect.w, h),
                Rect::new(rect.x, rect.y + h, rect.w, rect.h - h),
            )
        }
    }
}

impl<T: Clone + PartialEq> SplitTree<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Groups in layout order (left to right, top to bottom).
    pub fn groups(&self) -> Vec<&Group<T>> {
        let mut out = Vec::new();
        self.root.leaves(&mut out);
        out
    }

    pub fn group_count(&self) -> usize {
        self.groups().len()
    }

    pub fn is_split(&self) -> bool {
        self.group_count() > 1
    }

    pub fn group(&self, id: GroupId) -> Option<&Group<T>> {
        self.groups().into_iter().find(|group| group.id == id)
    }

    pub fn focused_id(&self) -> GroupId {
        self.focused
    }

    pub fn focused(&self) -> &Group<T> {
        self.group(self.focused)
            .expect("the focused group always exists")
    }

    pub fn group_of(&self, tab: &T) -> Option<GroupId> {
        self.groups()
            .into_iter()
            .find(|group| group.tabs.contains(tab))
            .map(|group| group.id)
    }

    pub fn contains(&self, tab: &T) -> bool {
        self.group_of(tab).is_some()
    }

    /// Focuses a group; false if it does not exist.
    pub fn focus(&mut self, id: GroupId) -> bool {
        let exists = self.group(id).is_some();
        if exists {
            self.focused = id;
        }
        exists
    }

    /// Focuses the next (or previous) group, wrapping around.
    pub fn focus_cycle(&mut self, forward: bool) {
        let ids: Vec<GroupId> = self.groups().iter().map(|g| g.id).collect();
        let at = ids.iter().position(|&id| id == self.focused).unwrap_or(0);
        let count = ids.len();
        self.focused = ids[if forward {
            (at + 1) % count
        } else {
            (at + count - 1) % count
        }];
    }

    /// The group shown "opposite" the focused one: the next group in layout
    /// order, wrapping; `None` when there is no split.
    pub fn opposite_of(&self, id: GroupId) -> Option<GroupId> {
        let ids: Vec<GroupId> = self.groups().iter().map(|g| g.id).collect();
        let at = ids.iter().position(|&g| g == id)?;
        (ids.len() > 1).then(|| ids[(at + 1) % ids.len()])
    }

    /// Adds `tab` to the focused group without changing focus; it becomes the
    /// group's active tab only when the group has none.
    pub fn add(&mut self, tab: T) {
        if self.contains(&tab) {
            return;
        }
        let focused = self.focused;
        let group = self.leaf(focused);
        if group.active.is_none() {
            group.active = Some(tab.clone());
        }
        group.tabs.push(tab);
    }

    /// Shows `tab`: activates it where it lives (focusing that group), or adds
    /// it to the focused group.
    pub fn open(&mut self, tab: T) {
        if let Some(id) = self.group_of(&tab) {
            self.focused = id;
        } else {
            let focused = self.focused;
            self.leaf(focused).tabs.push(tab.clone());
        }
        let focused = self.focused;
        self.leaf(focused).active = Some(tab);
    }

    /// Opens `tab` in the group opposite the focused one, creating a split to
    /// the right when there is none.
    pub fn open_in_opposite(&mut self, tab: T) -> GroupId {
        let target = match self.opposite_of(self.focused) {
            Some(id) => id,
            None => self.split(self.focused, Edge::Right, None),
        };
        self.move_tab(&tab, target);
        self.focus(target);
        target
    }

    /// Splits `group` at `edge`. With `tab`, the tab moves into the new group
    /// (refused when it is the group's only tab and would leave it empty);
    /// without one the new group starts empty. Focuses the new group.
    pub fn split(&mut self, group: GroupId, edge: Edge, tab: Option<T>) -> GroupId {
        if self.group(group).is_none() {
            return self.focused;
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut new_group = Group {
            id,
            tabs: Vec::new(),
            active: None,
        };
        if let Some(tab) = tab {
            let source = self.group_of(&tab);
            if let Some(source) = source {
                let single = self.group(source).is_some_and(|g| g.tabs.len() == 1);
                if single && source == group {
                    // Moving the only tab would leave an empty group behind.
                    new_group.tabs = Vec::new();
                } else {
                    self.take_tab(&tab);
                    new_group.tabs.push(tab.clone());
                    new_group.active = Some(tab);
                }
            } else {
                new_group.tabs.push(tab.clone());
                new_group.active = Some(tab);
            }
        }
        // The source may have collapsed away; the target survives unless it was
        // the (emptied) source, which `take_tab` keeps while it is the target.
        if self.group(group).is_none() {
            return self.focused;
        }
        let _ = self.root.split_leaf(group, edge, new_group);
        self.focused = id;
        id
    }

    /// Moves `tab` into `target` and makes it active there.
    pub fn move_tab(&mut self, tab: &T, target: GroupId) {
        if self.group(target).is_none() {
            return;
        }
        if self.group_of(tab) == Some(target) {
            self.leaf(target).active = Some(tab.clone());
            return;
        }
        self.take_tab_keeping(tab, target);
        let group = self.leaf(target);
        group.tabs.push(tab.clone());
        group.active = Some(tab.clone());
    }

    /// Moves the tab to the opposite group (creating a split when needed).
    pub fn move_to_opposite(&mut self, tab: &T) -> Option<GroupId> {
        let source = self.group_of(tab)?;
        let target = match self.opposite_of(source) {
            Some(id) => id,
            None => {
                if self.group(source)?.tabs.len() < 2 {
                    return None;
                }
                self.split(source, Edge::Right, None)
            }
        };
        self.move_tab(tab, target);
        self.focus(target);
        Some(target)
    }

    /// Removes a tab everywhere. A group left empty by it collapses unless it is
    /// the last one.
    pub fn remove_tab(&mut self, tab: &T) {
        self.take_tab(tab);
    }

    /// Keeps only the tabs `keep` accepts.
    pub fn retain(&mut self, keep: impl Fn(&T) -> bool) {
        let doomed: Vec<T> = self
            .groups()
            .iter()
            .flat_map(|g| g.tabs.iter().filter(|t| !keep(t)).cloned())
            .collect();
        for tab in doomed {
            self.take_tab(&tab);
        }
    }

    /// Merges `group` into its neighbour and focuses it; false when it is the
    /// only group.
    pub fn unsplit(&mut self, group: GroupId) -> bool {
        let ids: Vec<GroupId> = self.groups().iter().map(|g| g.id).collect();
        let Some(at) = ids.iter().position(|&id| id == group) else {
            return false;
        };
        if ids.len() < 2 {
            return false;
        }
        let into = if at > 0 { ids[at - 1] } else { ids[1] };
        let Some(source) = self.group(group).cloned() else {
            return false;
        };
        let was_focused = self.focused == group;
        self.root.remove_leaf(group);
        let target = self.leaf(into);
        for tab in source.tabs {
            if !target.tabs.contains(&tab) {
                target.tabs.push(tab);
            }
        }
        if was_focused {
            target.active = source.active.or(target.active.take());
            self.focused = into;
        }
        true
    }

    /// Collapses every split into one group, keeping the focused tab active.
    pub fn unsplit_all(&mut self) {
        let active = self.focused().active.clone();
        let mut tabs: Vec<T> = Vec::new();
        for group in self.groups() {
            for tab in &group.tabs {
                if !tabs.contains(tab) {
                    tabs.push(tab.clone());
                }
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.root = Node::Leaf(Group { id, tabs, active });
        self.focused = id;
    }

    /// Screen rectangle of every group inside `area`.
    pub fn layout(&self, area: Rect) -> Vec<(GroupId, Rect)> {
        let mut out = Vec::new();
        self.root.layout(area, &mut out);
        out
    }

    pub fn dividers(&self, area: Rect) -> Vec<Divider> {
        let mut out = Vec::new();
        self.root.dividers(area, &mut Vec::new(), &mut out);
        out
    }

    /// Sets the share of the first child of the split at `path`.
    pub fn set_ratio(&mut self, path: &[bool], value: f32) {
        let mut node = &mut self.root;
        for &second in path {
            let Node::Split {
                first, second: s, ..
            } = node
            else {
                return;
            };
            node = if second { s } else { first };
        }
        if let Node::Split { ratio, .. } = node {
            *ratio = value.clamp(0.1, 0.9);
        }
    }

    /// Flips the orientation of the split that contains `group`'s leaf.
    pub fn change_orientation(&mut self, group: GroupId) {
        fn walk<T>(node: &mut Node<T>, id: GroupId) -> bool {
            let Node::Split {
                orientation,
                first,
                second,
                ..
            } = node
            else {
                return false;
            };
            let direct = |n: &Node<T>| matches!(n, Node::Leaf(g) if g.id == id);
            if direct(first) || direct(second) {
                *orientation = match orientation {
                    Orientation::Columns => Orientation::Rows,
                    Orientation::Rows => Orientation::Columns,
                };
                return true;
            }
            walk(first, id) || walk(second, id)
        }
        walk(&mut self.root, group);
    }

    /// Repairs a deserialized tree: unique tabs and group ids, valid focus and
    /// active tabs, no empty group next to others except by choice.
    pub fn normalize(&mut self) {
        let mut seen: Vec<T> = Vec::new();
        let mut ids = Vec::new();
        let mut leaves = Vec::new();
        self.root.leaves_mut(&mut leaves);
        for group in leaves {
            group.tabs.retain(|tab| {
                let fresh = !seen.contains(tab);
                if fresh {
                    seen.push(tab.clone());
                }
                fresh
            });
            if group
                .active
                .as_ref()
                .is_some_and(|a| !group.tabs.contains(a))
            {
                group.active = group.tabs.first().cloned();
            }
            ids.push(group.id);
        }
        ids.sort_unstable();
        let unique = ids.windows(2).all(|pair| pair[0] != pair[1]);
        if !unique || ids.contains(&GroupId::MAX) {
            *self = Self::default();
            return;
        }
        self.next_id = self.next_id.max(ids.last().map_or(0, |id| id + 1));
        if self.group(self.focused).is_none() {
            self.focused = ids[0];
        }
    }

    fn leaf(&mut self, id: GroupId) -> &mut Group<T> {
        self.root
            .leaf_mut(id)
            .expect("group ids come from this tree")
    }

    fn take_tab(&mut self, tab: &T) {
        self.take_tab_keeping(tab, GroupId::MAX);
    }

    /// Removes `tab` from its group; the group collapses when empty (unless it
    /// is `keep` or the only group) and focus follows.
    fn take_tab_keeping(&mut self, tab: &T, keep: GroupId) {
        let Some(id) = self.group_of(tab) else {
            return;
        };
        let group = self.leaf(id);
        let at = group.tabs.iter().position(|t| t == tab).unwrap_or(0);
        group.tabs.retain(|t| t != tab);
        if group.active.as_ref() == Some(tab) {
            group.active = group.tabs.get(at).or_else(|| group.tabs.last()).cloned();
        }
        let empty = group.tabs.is_empty();
        if empty && id != keep && self.group_count() > 1 {
            let ids: Vec<GroupId> = self.groups().iter().map(|g| g.id).collect();
            let pos = ids.iter().position(|&g| g == id).unwrap_or(0);
            let neighbour = if pos > 0 { ids[pos - 1] } else { ids[1] };
            self.root.remove_leaf(id);
            if self.focused == id {
                self.focused = neighbour;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(tabs: &[&'static str]) -> SplitTree<&'static str> {
        let mut tree = SplitTree::new();
        for tab in tabs {
            tree.open(*tab);
        }
        tree
    }

    fn names(tree: &SplitTree<&'static str>) -> Vec<Vec<&'static str>> {
        tree.groups().iter().map(|g| g.tabs.clone()).collect()
    }

    #[test]
    fn open_adds_to_the_focused_group_and_activates() {
        let mut t = tree(&["a", "b"]);
        assert_eq!(t.focused().active, Some("b"));
        t.open("a");
        assert_eq!(names(&t), [["a", "b"]]);
        assert_eq!(t.focused().active, Some("a"));
    }

    #[test]
    fn split_right_moves_the_tab_into_a_new_focused_group() {
        let mut t = tree(&["a", "b", "c"]);
        let new = t.split(t.focused_id(), Edge::Right, Some("c"));
        assert_eq!(names(&t), [vec!["a", "b"], vec!["c"]]);
        assert_eq!(t.focused_id(), new);
        assert_eq!(t.group(0).unwrap().active, Some("b"), "old group remembers");
        t.focus(0);
        assert_eq!(t.focused().active, Some("b"));
    }

    #[test]
    fn split_left_and_up_place_the_new_group_first() {
        let mut t = tree(&["a", "b"]);
        t.split(0, Edge::Left, Some("b"));
        assert_eq!(names(&t), [vec!["b"], vec!["a"]]);
        let area = Rect::new(0.0, 0.0, 100.0, 50.0);
        let layout = t.layout(area);
        assert_eq!(layout[0].1, Rect::new(0.0, 0.0, 50.0, 50.0));
        t.unsplit_all();
        t.split(t.focused_id(), Edge::Bottom, None);
        let layout = t.layout(area);
        assert_eq!(layout[1].1, Rect::new(0.0, 25.0, 100.0, 25.0));
    }

    #[test]
    fn splitting_the_only_tab_creates_an_empty_group() {
        let mut t = tree(&["a"]);
        let new = t.split(0, Edge::Right, Some("a"));
        assert_eq!(names(&t), [vec!["a"], vec![]]);
        assert_eq!(t.focused_id(), new);
    }

    #[test]
    fn closing_the_last_tab_of_a_group_collapses_it() {
        let mut t = tree(&["a", "b"]);
        t.split(0, Edge::Right, Some("b"));
        t.remove_tab(&"b");
        assert_eq!(t.group_count(), 1);
        assert_eq!(t.focused_id(), 0);
        t.remove_tab(&"a");
        assert_eq!(t.group_count(), 1, "the last group stays");
        assert_eq!(t.focused().active, None);
    }

    #[test]
    fn removing_the_active_tab_activates_its_neighbour() {
        let mut t = tree(&["a", "b", "c"]);
        t.open("b");
        t.remove_tab(&"b");
        assert_eq!(t.focused().active, Some("c"));
        t.remove_tab(&"c");
        assert_eq!(t.focused().active, Some("a"));
    }

    #[test]
    fn move_to_opposite_group_creates_then_reuses_the_split() {
        let mut t = tree(&["a", "b", "c"]);
        let right = t.move_to_opposite(&"c").unwrap();
        assert_eq!(names(&t), [vec!["a", "b"], vec!["c"]]);
        assert_eq!(t.focused_id(), right);
        t.move_to_opposite(&"b");
        assert_eq!(names(&t), [vec!["a"], vec!["c", "b"]]);
        t.move_to_opposite(&"c");
        assert_eq!(names(&t), [vec!["a", "c"], vec!["b"]]);
        let mut single = tree(&["a"]);
        assert_eq!(single.move_to_opposite(&"a"), None);
    }

    #[test]
    fn moving_the_last_tab_out_collapses_its_group() {
        let mut t = tree(&["a", "b"]);
        t.split(0, Edge::Right, Some("b"));
        t.move_tab(&"b", 0);
        assert_eq!(t.group_count(), 1);
        assert_eq!(names(&t), [vec!["a", "b"]]);
    }

    #[test]
    fn open_in_opposite_splits_when_alone() {
        let mut t = tree(&["a", "b"]);
        let right = t.open_in_opposite("c");
        assert_eq!(names(&t), [vec!["a", "b"], vec!["c"]]);
        assert_eq!(t.focused_id(), right);
        t.focus(0);
        t.open_in_opposite("a");
        assert_eq!(names(&t), [vec!["b"], vec!["c", "a"]]);
    }

    #[test]
    fn unsplit_merges_into_the_neighbour_and_all_merges_everything() {
        let mut t = tree(&["a", "b", "c", "d"]);
        t.split(0, Edge::Right, Some("c"));
        t.split(t.focused_id(), Edge::Bottom, Some("d"));
        assert_eq!(t.group_count(), 3);
        let focused = t.focused_id();
        assert!(t.unsplit(focused));
        assert_eq!(names(&t), [vec!["a", "b"], vec!["c", "d"]]);
        assert_eq!(t.focused().active, Some("d"));
        t.focus(0);
        t.unsplit_all();
        assert_eq!(names(&t), [vec!["a", "b", "c", "d"]]);
        assert_eq!(t.focused().active, Some("b"));
        assert!(!t.unsplit(t.focused_id()));
    }

    #[test]
    fn focus_cycles_through_groups() {
        let mut t = tree(&["a", "b", "c"]);
        t.split(0, Edge::Right, Some("b"));
        t.split(t.focused_id(), Edge::Right, Some("c"));
        t.focus(0);
        t.focus_cycle(true);
        assert_eq!(t.focused().tabs, ["b"]);
        t.focus_cycle(false);
        t.focus_cycle(false);
        assert_eq!(t.focused().tabs, ["c"], "wraps around");
    }

    #[test]
    fn retain_prunes_tabs_and_collapses_groups() {
        let mut t = tree(&["a", "b", "c"]);
        t.split(0, Edge::Right, Some("c"));
        t.retain(|tab| *tab != "c");
        assert_eq!(names(&t), [vec!["a", "b"]]);
    }

    #[test]
    fn drop_edges_pick_the_nearest_edge_zone() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(drop_edge(r, 5.0, 50.0, 0.2), Some(Edge::Left));
        assert_eq!(drop_edge(r, 95.0, 50.0, 0.2), Some(Edge::Right));
        assert_eq!(drop_edge(r, 50.0, 3.0, 0.2), Some(Edge::Top));
        assert_eq!(drop_edge(r, 50.0, 90.0, 0.2), Some(Edge::Bottom));
        assert_eq!(drop_edge(r, 15.0, 5.0, 0.2), Some(Edge::Top));
        assert_eq!(drop_edge(r, 50.0, 50.0, 0.2), None);
        assert_eq!(drop_edge(r, 150.0, 50.0, 0.2), None);
    }

    #[test]
    fn ratio_and_orientation_can_change() {
        let mut t = tree(&["a", "b"]);
        t.split(0, Edge::Right, Some("b"));
        let area = Rect::new(0.0, 0.0, 200.0, 100.0);
        let dividers = t.dividers(area);
        assert_eq!(dividers.len(), 1);
        t.set_ratio(&dividers[0].path, 0.25);
        assert_eq!(t.layout(area)[0].1.w, 50.0);
        t.set_ratio(&[], 5.0);
        assert_eq!(t.dividers(area)[0].ratio, 0.9);
        t.change_orientation(0);
        assert_eq!(t.dividers(area)[0].orientation, Orientation::Rows);
    }

    #[test]
    fn normalize_repairs_duplicates_and_stale_state() {
        let mut t = tree(&["a", "b"]);
        t.split(0, Edge::Right, Some("b"));
        t.leaf(0).tabs.push("b");
        t.leaf(0).active = Some("zzz");
        t.focused = 99;
        t.normalize();
        assert_eq!(names(&t), [vec!["a", "b"], vec![]]);
        assert_eq!(t.focused_id(), 0);
        assert_eq!(t.group(0).unwrap().active, Some("a"));
    }
}
