//! Extension points (IntelliJ: `ExtensionPointName`, `ExtensionsArea`, `LoadingOrder`).
//!
//! An [`ExtensionPointName<T>`] names a place where features contribute implementations of `T`
//! (usually `dyn Trait`): language definitions, inspections, run configuration types and so
//! on. Extensions are id-addressable and ordered: by registration, or by a [`LoadingOrder`]
//! (`First`, `Last`, `Before(id)`, `After(id)`). An extension registered under a parent
//! [`DisposableId`] is removed when the parent is disposed, which is how a plugin unload
//! takes its contributions back.

use crate::disposer::{DisposableId, Disposed, Disposer};
use parking_lot::Mutex;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

/// The name of an extension point whose extensions are of type `T`.
pub struct ExtensionPointName<T: ?Sized + 'static> {
    name: &'static str,
    marker: PhantomData<fn() -> Box<T>>,
}

impl<T: ?Sized + 'static> ExtensionPointName<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    fn key(&self) -> PointKey {
        PointKey {
            extension: TypeId::of::<T>(),
            name: self.name,
        }
    }
}

impl<T: ?Sized + 'static> Clone for ExtensionPointName<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + 'static> Copy for ExtensionPointName<T> {}

impl<T: ?Sized + 'static> fmt::Debug for ExtensionPointName<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExtensionPointName({})", self.name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PointKey {
    extension: TypeId,
    name: &'static str,
}

/// Where an extension goes in the ordered list of its extension point.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LoadingOrder {
    /// In registration order, after the `First` ones and before the `Last` ones.
    #[default]
    Anywhere,
    First,
    Last,
    /// Directly before the extension with this id (ignored if that id is not registered).
    Before(String),
    /// Directly after the extension with this id (ignored if that id is not registered).
    After(String),
}

/// An extension id is already taken on that extension point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionError {
    DuplicateId { point: &'static str, id: String },
    ParentDisposed(Disposed),
}

impl fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId { point, id } => {
                write!(
                    f,
                    "extension point `{point}` already has an extension `{id}`"
                )
            }
            Self::ParentDisposed(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ExtensionError {}

struct Entry {
    id: String,
    order: LoadingOrder,
    /// An `Arc<T>` for the point's extension type.
    extension: Arc<dyn Any + Send + Sync>,
}

type ChangeListener = (u64, Arc<dyn Fn() + Send + Sync>);

#[derive(Default)]
struct Point {
    /// In registration order.
    entries: Vec<Entry>,
    listeners: Vec<ChangeListener>,
}

#[derive(Default)]
struct Inner {
    points: HashMap<PointKey, Point>,
    next_listener: u64,
}

/// All extension points of an application. Cloning gives another handle to the same registry.
#[derive(Clone)]
pub struct ExtensionRegistry {
    inner: Arc<Mutex<Inner>>,
    disposer: Disposer,
}

impl fmt::Debug for ExtensionRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExtensionRegistry({} points)",
            self.inner.lock().points.len()
        )
    }
}

impl ExtensionRegistry {
    pub fn new(disposer: Disposer) -> Self {
        Self {
            inner: Arc::default(),
            disposer,
        }
    }

    /// Adds an extension. With a `parent` it is removed again when the parent is disposed.
    pub fn register<T: ?Sized + Send + Sync + 'static>(
        &self,
        point: ExtensionPointName<T>,
        id: impl Into<String>,
        order: LoadingOrder,
        extension: Arc<T>,
        parent: Option<DisposableId>,
    ) -> Result<(), ExtensionError> {
        let id = id.into();
        let key = point.key();
        // Claim the parent first so a disposed parent leaves no half-registered extension.
        let node = match parent {
            Some(parent) => Some(
                self.disposer
                    .new_child(parent, &format!("extension {id}"))
                    .map_err(ExtensionError::ParentDisposed)?,
            ),
            None => None,
        };
        let listeners = {
            let mut inner = self.inner.lock();
            let entry = inner.points.entry(key).or_default();
            if entry.entries.iter().any(|existing| existing.id == id) {
                drop(inner);
                if let Some(node) = node {
                    self.disposer.dispose(node);
                }
                return Err(ExtensionError::DuplicateId {
                    point: point.name,
                    id,
                });
            }
            entry.entries.push(Entry {
                id: id.clone(),
                order,
                extension: Arc::new(extension),
            });
            entry.listeners.clone()
        };
        if let Some(node) = node {
            let registry = self.clone();
            let _ = self.disposer.register_fn(node, "unregister", move || {
                registry.unregister(point, &id);
            });
        }
        notify(&listeners);
        Ok(())
    }

    /// Removes the extension with `id`; returns whether there was one.
    pub fn unregister<T: ?Sized + 'static>(&self, point: ExtensionPointName<T>, id: &str) -> bool {
        let listeners = {
            let mut inner = self.inner.lock();
            let Some(entry) = inner.points.get_mut(&point.key()) else {
                return false;
            };
            let Some(index) = entry.entries.iter().position(|e| e.id == id) else {
                return false;
            };
            entry.entries.remove(index);
            entry.listeners.clone()
        };
        notify(&listeners);
        true
    }

    /// The extensions in loading order (IntelliJ `extensionList`).
    pub fn extension_list<T: ?Sized + Send + Sync + 'static>(
        &self,
        point: ExtensionPointName<T>,
    ) -> Vec<Arc<T>> {
        self.extensions_with_ids(point)
            .into_iter()
            .map(|(_, extension)| extension)
            .collect()
    }

    /// Ids and extensions in loading order.
    pub fn extensions_with_ids<T: ?Sized + Send + Sync + 'static>(
        &self,
        point: ExtensionPointName<T>,
    ) -> Vec<(String, Arc<T>)> {
        let inner = self.inner.lock();
        let Some(entry) = inner.points.get(&point.key()) else {
            return Vec::new();
        };
        sorted(&entry.entries)
            .into_iter()
            .filter_map(|e| {
                e.extension
                    .downcast_ref::<Arc<T>>()
                    .map(|extension| (e.id.clone(), extension.clone()))
            })
            .collect()
    }

    /// The extension with `id` (IntelliJ `findExtension` by id).
    pub fn find_by_id<T: ?Sized + Send + Sync + 'static>(
        &self,
        point: ExtensionPointName<T>,
        id: &str,
    ) -> Option<Arc<T>> {
        let inner = self.inner.lock();
        inner
            .points
            .get(&point.key())?
            .entries
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.extension.downcast_ref::<Arc<T>>().cloned())
    }

    /// Ids in loading order.
    pub fn ids<T: ?Sized + 'static>(&self, point: ExtensionPointName<T>) -> Vec<String> {
        let inner = self.inner.lock();
        inner
            .points
            .get(&point.key())
            .map(|entry| {
                sorted(&entry.entries)
                    .into_iter()
                    .map(|e| e.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Calls `listener` after every addition or removal on `point`, until `parent` is
    /// disposed (IntelliJ `addChangeListener`).
    pub fn add_change_listener<T: ?Sized + 'static>(
        &self,
        point: ExtensionPointName<T>,
        parent: DisposableId,
        listener: impl Fn() + Send + Sync + 'static,
    ) -> Result<(), Disposed> {
        let key = point.key();
        let listener_id = {
            let mut inner = self.inner.lock();
            inner.next_listener += 1;
            let listener_id = inner.next_listener;
            inner
                .points
                .entry(key)
                .or_default()
                .listeners
                .push((listener_id, Arc::new(listener)));
            listener_id
        };
        let inner = Arc::downgrade(&self.inner);
        let result = self
            .disposer
            .register_fn(parent, "extension listener", move || {
                if let Some(inner) = inner.upgrade()
                    && let Some(entry) = inner.lock().points.get_mut(&key)
                {
                    entry.listeners.retain(|(id, _)| *id != listener_id);
                }
            });
        if result.is_err()
            && let Some(entry) = self.inner.lock().points.get_mut(&key)
        {
            entry.listeners.retain(|(id, _)| *id != listener_id);
        }
        result.map(|_| ())
    }
}

fn notify(listeners: &[ChangeListener]) {
    for (_, listener) in listeners {
        listener();
    }
}

/// Orders entries (given in registration order): `First` ones, then `Anywhere`, then `Last`;
/// afterwards each `Before`/`After` entry is placed next to its target, in registration order,
/// repeating so that a chain of relative entries resolves. Entries whose target never appears
/// stay among the `Anywhere` ones.
fn sorted(entries: &[Entry]) -> Vec<&Entry> {
    let class = |e: &&Entry| match e.order {
        LoadingOrder::First => 0,
        LoadingOrder::Last => 2,
        _ => 1,
    };
    let relative = |e: &&Entry| matches!(e.order, LoadingOrder::Before(_) | LoadingOrder::After(_));
    let mut list: Vec<&Entry> = Vec::with_capacity(entries.len());
    for group in 0..3 {
        list.extend(entries.iter().filter(|e| class(e) == group && !relative(e)));
    }
    let mut pending: Vec<&Entry> = entries.iter().filter(relative).collect();
    loop {
        let before = pending.len();
        let mut unresolved = Vec::new();
        for entry in pending {
            let (target, offset) = match &entry.order {
                LoadingOrder::Before(id) => (id, 0),
                LoadingOrder::After(id) => (id, 1),
                _ => unreachable!("only relative entries are pending"),
            };
            match list.iter().position(|e| e.id == *target) {
                Some(index) => list.insert(index + offset, entry),
                None => unresolved.push(entry),
            }
        }
        pending = unresolved;
        if pending.is_empty() || pending.len() == before {
            break;
        }
    }
    // Targets that never appeared: keep them, after the `Anywhere` entries.
    let end_of_anywhere = list
        .iter()
        .rposition(|e| !matches!(e.order, LoadingOrder::Last))
        .map_or(0, |index| index + 1);
    for (offset, entry) in pending.into_iter().enumerate() {
        list.insert(end_of_anywhere + offset, entry);
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    trait Inspection: Send + Sync {
        fn name(&self) -> String;
    }

    struct Named(&'static str);
    impl Inspection for Named {
        fn name(&self) -> String {
            self.0.to_owned()
        }
    }

    const INSPECTIONS: ExtensionPointName<dyn Inspection> = ExtensionPointName::new("inspection");

    fn registry() -> (Disposer, ExtensionRegistry) {
        let disposer = Disposer::new();
        (disposer.clone(), ExtensionRegistry::new(disposer))
    }

    fn add(registry: &ExtensionRegistry, id: &'static str, order: LoadingOrder) {
        registry
            .register(INSPECTIONS, id, order, Arc::new(Named(id)), None)
            .unwrap();
    }

    #[test]
    fn extensions_keep_registration_order_and_are_found_by_id() {
        let (_, registry) = registry();
        add(&registry, "a", LoadingOrder::Anywhere);
        add(&registry, "b", LoadingOrder::Anywhere);
        add(&registry, "c", LoadingOrder::Anywhere);
        let names: Vec<_> = registry
            .extension_list(INSPECTIONS)
            .iter()
            .map(|e| e.name())
            .collect();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!(registry.find_by_id(INSPECTIONS, "b").unwrap().name(), "b");
        assert!(registry.find_by_id(INSPECTIONS, "zzz").is_none());
        assert_eq!(registry.ids(INSPECTIONS), ["a", "b", "c"]);
    }

    #[test]
    fn first_and_last_override_registration_order() {
        let (_, registry) = registry();
        add(&registry, "mid1", LoadingOrder::Anywhere);
        add(&registry, "last", LoadingOrder::Last);
        add(&registry, "first", LoadingOrder::First);
        add(&registry, "mid2", LoadingOrder::Anywhere);
        assert_eq!(registry.ids(INSPECTIONS), ["first", "mid1", "mid2", "last"]);
    }

    #[test]
    fn before_and_after_place_next_to_their_target() {
        let (_, registry) = registry();
        add(&registry, "x", LoadingOrder::Before("c".into()));
        add(&registry, "a", LoadingOrder::Anywhere);
        add(&registry, "b", LoadingOrder::Anywhere);
        add(&registry, "c", LoadingOrder::Anywhere);
        add(&registry, "y", LoadingOrder::After("a".into()));
        assert_eq!(registry.ids(INSPECTIONS), ["a", "y", "b", "x", "c"]);
    }

    #[test]
    fn relative_entries_can_chain_and_unknown_targets_are_ignored() {
        let (_, registry) = registry();
        add(&registry, "second", LoadingOrder::After("first".into()));
        add(&registry, "first", LoadingOrder::After("base".into()));
        add(&registry, "base", LoadingOrder::Anywhere);
        add(&registry, "lost", LoadingOrder::Before("missing".into()));
        add(&registry, "end", LoadingOrder::Last);
        assert_eq!(
            registry.ids(INSPECTIONS),
            ["base", "first", "second", "lost", "end"]
        );
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let (_, registry) = registry();
        add(&registry, "a", LoadingOrder::Anywhere);
        let again = registry.register(
            INSPECTIONS,
            "a",
            LoadingOrder::Anywhere,
            Arc::new(Named("other")),
            None,
        );
        assert!(matches!(again, Err(ExtensionError::DuplicateId { .. })));
        assert_eq!(registry.extension_list(INSPECTIONS).len(), 1);
    }

    #[test]
    fn unregister_removes_the_extension() {
        let (_, registry) = registry();
        add(&registry, "a", LoadingOrder::Anywhere);
        assert!(registry.unregister(INSPECTIONS, "a"));
        assert!(!registry.unregister(INSPECTIONS, "a"));
        assert!(registry.extension_list(INSPECTIONS).is_empty());
    }

    #[test]
    fn extensions_registered_under_a_parent_go_away_with_it() {
        let (disposer, registry) = registry();
        let plugin = disposer.new_disposable("plugin");
        add(&registry, "builtin", LoadingOrder::Anywhere);
        registry
            .register(
                INSPECTIONS,
                "from plugin",
                LoadingOrder::Anywhere,
                Arc::new(Named("p")),
                Some(plugin),
            )
            .unwrap();
        assert_eq!(registry.ids(INSPECTIONS), ["builtin", "from plugin"]);
        disposer.dispose(plugin);
        assert_eq!(registry.ids(INSPECTIONS), ["builtin"]);
        let late = registry.register(
            INSPECTIONS,
            "late",
            LoadingOrder::Anywhere,
            Arc::new(Named("l")),
            Some(plugin),
        );
        assert!(matches!(late, Err(ExtensionError::ParentDisposed(_))));
    }

    #[test]
    fn a_rejected_duplicate_does_not_leak_its_parent_node() {
        let (disposer, registry) = registry();
        let plugin = disposer.new_disposable("plugin");
        add(&registry, "a", LoadingOrder::Anywhere);
        let _ = registry.register(
            INSPECTIONS,
            "a",
            LoadingOrder::Anywhere,
            Arc::new(Named("x")),
            Some(plugin),
        );
        assert!(disposer.children(plugin).is_empty());
    }

    #[test]
    fn change_listeners_hear_additions_and_removals_until_disposed() {
        let (disposer, registry) = registry();
        let owner = disposer.new_disposable("owner");
        let changes = Arc::new(AtomicUsize::new(0));
        let probe = changes.clone();
        registry
            .add_change_listener(INSPECTIONS, owner, move || {
                probe.fetch_add(1, Ordering::SeqCst);
            })
            .unwrap();
        add(&registry, "a", LoadingOrder::Anywhere);
        registry.unregister(INSPECTIONS, "a");
        assert_eq!(changes.load(Ordering::SeqCst), 2);
        disposer.dispose(owner);
        add(&registry, "b", LoadingOrder::Anywhere);
        assert_eq!(changes.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn points_of_other_types_or_names_are_separate() {
        const OTHER: ExtensionPointName<dyn Inspection> = ExtensionPointName::new("other");
        let (_, registry) = registry();
        add(&registry, "a", LoadingOrder::Anywhere);
        assert!(registry.extension_list(OTHER).is_empty());
        assert_eq!(OTHER.name(), "other");
    }
}
