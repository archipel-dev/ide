//! `Disposable` and the `Disposer` tree (IntelliJ: `Disposable`, `Disposer`).
//!
//! Resources that must be released together hang under a common parent. Disposing a parent
//! disposes its children first, newest registration first, and the parent last, so a child can
//! still use what its parent owns while it shuts down. Everything is idempotent: disposing a
//! node that is already gone does nothing.
//!
//! Unlike IntelliJ's static `Disposer`, a [`Disposer`] is an ordinary value (one per
//! [`Application`](crate::Application)); there is no global mutable state.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

/// Something that releases resources when its owner goes away.
pub trait Disposable: Send + Sync {
    /// Called at most once, after all children of the node it was registered under.
    fn dispose(&self);
}

/// Identifies a node of the [`Disposer`] tree. Ids are never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DisposableId(u64);

/// The parent was already disposed (or never existed), so nothing can be registered under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disposed(pub DisposableId);

impl fmt::Display for Disposed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parent disposable #{} is already disposed", self.0.0)
    }
}

impl std::error::Error for Disposed {}

struct Node {
    name: String,
    parent: Option<DisposableId>,
    /// In registration order.
    children: Vec<DisposableId>,
    object: Option<Arc<dyn Disposable>>,
}

#[derive(Default)]
struct Tree {
    next: u64,
    nodes: HashMap<DisposableId, Node>,
}

impl Tree {
    fn insert(
        &mut self,
        parent: Option<DisposableId>,
        name: &str,
        object: Option<Arc<dyn Disposable>>,
    ) -> Result<DisposableId, Disposed> {
        if let Some(parent) = parent
            && !self.nodes.contains_key(&parent)
        {
            return Err(Disposed(parent));
        }
        self.next += 1;
        let id = DisposableId(self.next);
        if let Some(parent) = parent {
            self.nodes
                .get_mut(&parent)
                .expect("checked above")
                .children
                .push(id);
        }
        self.nodes.insert(
            id,
            Node {
                name: name.to_owned(),
                parent,
                children: Vec::new(),
                object,
            },
        );
        Ok(id)
    }
}

/// The tree of disposables. Cloning gives another handle to the same tree.
#[derive(Clone, Default)]
pub struct Disposer {
    tree: Arc<Mutex<Tree>>,
}

impl fmt::Debug for Disposer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Disposer")
            .field("live", &self.tree.lock().nodes.len())
            .finish()
    }
}

struct FnDisposable(Mutex<Option<Box<dyn FnOnce() + Send>>>);

impl Disposable for FnDisposable {
    fn dispose(&self) {
        if let Some(action) = self.0.lock().take() {
            action();
        }
    }
}

impl Disposer {
    pub fn new() -> Self {
        Self::default()
    }

    /// A new root node (IntelliJ `Disposer.newDisposable()`): it owns nothing itself, but
    /// children can be registered under it and disposed with it.
    pub fn new_disposable(&self, name: &str) -> DisposableId {
        self.tree
            .lock()
            .insert(None, name, None)
            .expect("a root has no parent")
    }

    /// A new plain node under `parent` (IntelliJ `Disposer.newDisposable(parent)`).
    pub fn new_child(&self, parent: DisposableId, name: &str) -> Result<DisposableId, Disposed> {
        self.tree.lock().insert(Some(parent), name, None)
    }

    /// Registers `object` under `parent` (IntelliJ `Disposer.register(parent, child)`).
    pub fn register(
        &self,
        parent: DisposableId,
        name: &str,
        object: Arc<dyn Disposable>,
    ) -> Result<DisposableId, Disposed> {
        self.tree.lock().insert(Some(parent), name, Some(object))
    }

    /// Registers a closure that runs once when `parent` (or the returned node) is disposed.
    pub fn register_fn(
        &self,
        parent: DisposableId,
        name: &str,
        action: impl FnOnce() + Send + 'static,
    ) -> Result<DisposableId, Disposed> {
        self.register(
            parent,
            name,
            Arc::new(FnDisposable(Mutex::new(Some(Box::new(action))))),
        )
    }

    /// Disposes `id` and everything below it. Children go first, newest registration first;
    /// `id` itself goes last and is then detached from its parent. Returns whether the node
    /// was still alive.
    ///
    /// A panic in one `dispose` is contained so the others still run.
    pub fn dispose(&self, id: DisposableId) -> bool {
        let objects = {
            let mut tree = self.tree.lock();
            if !tree.nodes.contains_key(&id) {
                return false;
            }
            if let Some(parent) = tree.nodes.get(&id).and_then(|node| node.parent)
                && let Some(parent) = tree.nodes.get_mut(&parent)
            {
                parent.children.retain(|child| *child != id);
            }
            // Post-order: children (newest first) before their parent. Removing the nodes here
            // lets `dispose` callbacks use the disposer without deadlocking.
            let mut order = Vec::new();
            let mut stack = vec![(id, false)];
            while let Some((node, expanded)) = stack.pop() {
                if expanded {
                    order.push(node);
                } else {
                    stack.push((node, true));
                    if let Some(entry) = tree.nodes.get(&node) {
                        // Pushed oldest first so the newest child is popped (visited) first.
                        for child in &entry.children {
                            stack.push((*child, false));
                        }
                    }
                }
            }
            order
                .into_iter()
                .filter_map(|node| tree.nodes.remove(&node))
                .filter_map(|node| node.object.map(|object| (node.name, object)))
                .collect::<Vec<_>>()
        };
        for (name, object) in objects {
            if catch_unwind(AssertUnwindSafe(|| object.dispose())).is_err() {
                eprintln!("archipel-ide-platform: disposing `{name}` panicked");
            }
        }
        true
    }

    /// Disposes the children of `id` but keeps `id` itself.
    pub fn dispose_children(&self, id: DisposableId) {
        for child in self.children(id).into_iter().rev() {
            self.dispose(child);
        }
    }

    pub fn is_disposed(&self, id: DisposableId) -> bool {
        !self.tree.lock().nodes.contains_key(&id)
    }

    /// The live children of `id` in registration order (empty if `id` is disposed).
    pub fn children(&self, id: DisposableId) -> Vec<DisposableId> {
        self.tree
            .lock()
            .nodes
            .get(&id)
            .map(|node| node.children.clone())
            .unwrap_or_default()
    }

    /// The debug name given at registration.
    pub fn name(&self, id: DisposableId) -> Option<String> {
        self.tree
            .lock()
            .nodes
            .get(&id)
            .map(|node| node.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recorder() -> (
        Arc<Mutex<Vec<String>>>,
        impl Fn(&'static str) -> Arc<dyn Disposable>,
    ) {
        struct Named(&'static str, Arc<Mutex<Vec<String>>>);
        impl Disposable for Named {
            fn dispose(&self) {
                self.1.lock().push(self.0.to_owned());
            }
        }
        let log = Arc::new(Mutex::new(Vec::new()));
        let shared = log.clone();
        (log, move |name| Arc::new(Named(name, shared.clone())))
    }

    #[test]
    fn children_are_disposed_before_the_parent_newest_first() {
        let disposer = Disposer::new();
        let (log, named) = recorder();
        let root = disposer.new_disposable("root");
        let parent = disposer.register(root, "parent", named("parent")).unwrap();
        disposer.register(parent, "a", named("a")).unwrap();
        let b = disposer.register(parent, "b", named("b")).unwrap();
        disposer.register(b, "b1", named("b1")).unwrap();
        disposer.register(b, "b2", named("b2")).unwrap();
        disposer
            .register(root, "sibling", named("sibling"))
            .unwrap();

        assert!(disposer.dispose(root));
        assert_eq!(
            *log.lock(),
            ["sibling", "b2", "b1", "b", "a", "parent"],
            "newest first at every level, parents after their children"
        );
    }

    #[test]
    fn disposing_twice_runs_dispose_once() {
        let disposer = Disposer::new();
        let (log, named) = recorder();
        let root = disposer.new_disposable("root");
        let child = disposer.register(root, "c", named("c")).unwrap();
        assert!(disposer.dispose(child));
        assert!(!disposer.dispose(child));
        assert!(disposer.dispose(root));
        assert_eq!(*log.lock(), ["c"]);
    }

    #[test]
    fn disposing_a_child_detaches_it_from_its_parent() {
        let disposer = Disposer::new();
        let (log, named) = recorder();
        let root = disposer.new_disposable("root");
        let a = disposer.register(root, "a", named("a")).unwrap();
        disposer.register(root, "b", named("b")).unwrap();
        disposer.dispose(a);
        assert_eq!(disposer.children(root).len(), 1);
        disposer.dispose(root);
        assert_eq!(*log.lock(), ["a", "b"]);
    }

    #[test]
    fn registering_under_a_disposed_parent_fails() {
        let disposer = Disposer::new();
        let root = disposer.new_disposable("root");
        disposer.dispose(root);
        assert!(disposer.is_disposed(root));
        assert_eq!(disposer.new_child(root, "late"), Err(Disposed(root)));
        assert!(disposer.register_fn(root, "late", || {}).is_err());
    }

    #[test]
    fn closures_run_once_and_may_use_the_disposer() {
        let disposer = Disposer::new();
        let root = disposer.new_disposable("root");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (inner, probe) = (disposer.clone(), seen.clone());
        disposer
            .register_fn(root, "probe", move || {
                // The node is already detached, so asking about the tree cannot deadlock.
                probe.lock().push(inner.is_disposed(root));
            })
            .unwrap();
        disposer.dispose(root);
        assert_eq!(*seen.lock(), [true]);
    }

    #[test]
    fn a_panicking_dispose_does_not_stop_the_others() {
        struct Boom;
        impl Disposable for Boom {
            fn dispose(&self) {
                panic!("boom");
            }
        }
        let disposer = Disposer::new();
        let (log, named) = recorder();
        let root = disposer.new_disposable("root");
        disposer.register(root, "first", named("first")).unwrap();
        disposer.register(root, "boom", Arc::new(Boom)).unwrap();
        disposer.register(root, "last", named("last")).unwrap();
        disposer.dispose(root);
        assert_eq!(*log.lock(), ["last", "first"]);
    }

    #[test]
    fn dispose_children_keeps_the_node() {
        let disposer = Disposer::new();
        let (log, named) = recorder();
        let root = disposer.new_disposable("root");
        disposer.register(root, "a", named("a")).unwrap();
        disposer.dispose_children(root);
        assert!(!disposer.is_disposed(root));
        assert!(disposer.children(root).is_empty());
        assert_eq!(*log.lock(), ["a"]);
        assert_eq!(disposer.name(root).as_deref(), Some("root"));
    }
}
