//! Typed, language-keyed extension registry (IntelliJ's `LanguageExtension`).
//!
//! Services are trait objects registered for a language or for the "any
//! language" slot ([`LanguageId::ANY`]). [`ServiceRegistry::get`] returns the
//! first service for the language, then its base-language chain, then "any";
//! [`ServiceRegistry::all`] returns the union in that order, deduplicated by
//! service id (the most specific registration wins), sorted by the explicit
//! [`Order`] constraints (`First`, `Last`, `Before(id)`, `After(id)`) and
//! otherwise by registration order. Generic tree-sitter services register
//! for "any"; LSP-backed providers use `Order::Last` so native ones win.

use crate::ids::LanguageId;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Placement of a service among the others of the same slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Order {
    First,
    #[default]
    Default,
    Last,
    /// Before the service with this id.
    Before(&'static str),
    /// After the service with this id.
    After(&'static str),
}

/// Metadata of one registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Registration {
    /// Stable id (used by `Before`/`After`, settings and dedupe).
    pub id: &'static str,
    pub language: LanguageId,
    pub order: Order,
    /// Usable while indexes are not ready (IntelliJ `DumbAware`).
    pub dumb_aware: bool,
}

impl Registration {
    pub fn new(id: &'static str, language: LanguageId) -> Self {
        Self {
            id,
            language,
            order: Order::Default,
            dumb_aware: false,
        }
    }

    /// A registration for every language.
    pub fn any(id: &'static str) -> Self {
        Self::new(id, LanguageId::ANY)
    }

    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    pub fn dumb_aware(mut self, dumb_aware: bool) -> Self {
        self.dumb_aware = dumb_aware;
        self
    }
}

#[derive(Clone)]
struct Entry {
    registration: Registration,
    seq: usize,
    /// An `Arc<T>` for the slot's `T` (shared, so the registry can be cloned: a clone holds the
    /// same service instances).
    service: Arc<dyn Any + Send + Sync>,
}

/// See the module docs. Cloning shares the service instances.
#[derive(Default, Clone)]
pub struct ServiceRegistry {
    slots: HashMap<TypeId, Vec<Entry>>,
    bases: HashMap<LanguageId, LanguageId>,
    seq: usize,
}

impl std::fmt::Debug for ServiceRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceRegistry")
            .field("slots", &self.slots.len())
            .field("bases", &self.bases)
            .finish()
    }
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Declares `base` as the base language of `language` (TypeScript ->
    /// JavaScript). Lookups fall back along the chain.
    pub fn declare_base(&mut self, language: LanguageId, base: LanguageId) {
        if language != base {
            self.bases.insert(language, base);
        }
    }

    pub fn base_of(&self, language: LanguageId) -> Option<LanguageId> {
        self.bases.get(&language).copied()
    }

    /// `language`, its base chain (cycle-safe), then [`LanguageId::ANY`].
    pub fn language_chain(&self, language: LanguageId) -> Vec<LanguageId> {
        let mut chain = vec![language];
        let mut current = language;
        while let Some(base) = self.base_of(current) {
            if chain.contains(&base) {
                break;
            }
            chain.push(base);
            current = base;
        }
        if !language.is_any() {
            chain.push(LanguageId::ANY);
        }
        chain
    }

    /// Registers `service` in the slot of `T` (usually a `dyn Trait`).
    pub fn register<T: ?Sized + Send + Sync + 'static>(
        &mut self,
        registration: Registration,
        service: Arc<T>,
    ) {
        self.seq += 1;
        self.slots
            .entry(TypeId::of::<T>())
            .or_default()
            .push(Entry {
                registration,
                seq: self.seq,
                service: Arc::new(service),
            });
    }

    /// Number of registrations of `T` (all languages).
    pub fn count<T: ?Sized + 'static>(&self) -> usize {
        self.slots.get(&TypeId::of::<T>()).map_or(0, Vec::len)
    }

    fn stage<T: ?Sized + 'static>(&self, language: LanguageId) -> Vec<&Entry> {
        let mut entries: Vec<&Entry> = self
            .slots
            .get(&TypeId::of::<T>())
            .into_iter()
            .flatten()
            .filter(|entry| entry.registration.language == language)
            .collect();
        entries.sort_by_key(|entry| (bucket(entry.registration.order), entry.seq));
        order_entries(entries)
    }

    /// The first service for `language`, its base chain, then "any".
    pub fn get<T: ?Sized + Send + Sync + 'static>(&self, language: LanguageId) -> Option<Arc<T>> {
        self.language_chain(language)
            .into_iter()
            .find_map(|language| self.stage::<T>(language).first().and_then(|e| arc(e)))
    }

    /// Every service for `language` (see the module docs for the order).
    pub fn all<T: ?Sized + Send + Sync + 'static>(&self, language: LanguageId) -> Vec<Arc<T>> {
        self.entries::<T>(language)
            .into_iter()
            .map(|(_, service)| service)
            .collect()
    }

    /// Like [`ServiceRegistry::all`]; while `dumb` only dumb-aware services.
    pub fn all_available<T: ?Sized + Send + Sync + 'static>(
        &self,
        language: LanguageId,
        dumb: bool,
    ) -> Vec<Arc<T>> {
        self.entries::<T>(language)
            .into_iter()
            .filter(|(registration, _)| !dumb || registration.dumb_aware)
            .map(|(_, service)| service)
            .collect()
    }

    /// Registrations and services for `language`, ordered.
    pub fn entries<T: ?Sized + Send + Sync + 'static>(
        &self,
        language: LanguageId,
    ) -> Vec<(Registration, Arc<T>)> {
        let mut seen = std::collections::HashSet::new();
        let mut entries: Vec<(usize, &Entry)> = Vec::new();
        for (stage, language) in self.language_chain(language).into_iter().enumerate() {
            for entry in self.slots.get(&TypeId::of::<T>()).into_iter().flatten() {
                if entry.registration.language == language && seen.insert(entry.registration.id) {
                    entries.push((stage, entry));
                }
            }
        }
        entries.sort_by_key(|(stage, entry)| (bucket(entry.registration.order), *stage, entry.seq));
        order_entries(entries.into_iter().map(|(_, entry)| entry).collect())
            .into_iter()
            .filter_map(|entry| Some((entry.registration, arc::<T>(entry)?)))
            .collect()
    }
}

fn arc<T: ?Sized + Send + Sync + 'static>(entry: &Entry) -> Option<Arc<T>> {
    (*entry.service).downcast_ref::<Arc<T>>().cloned()
}

fn bucket(order: Order) -> u8 {
    match order {
        Order::First => 0,
        Order::Last => 2,
        _ => 1,
    }
}

/// Stable topological sort: `entries` arrive in their base order; explicit
/// `Before`/`After` constraints move entries; unknown ids are ignored and
/// cycles fall back to the base order.
fn order_entries(entries: Vec<&Entry>) -> Vec<&Entry> {
    let count = entries.len();
    let index_of = |id: &str| entries.iter().position(|entry| entry.registration.id == id);
    // edges[a] contains b: a must come before b.
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut indegree = vec![0usize; count];
    let mut add = |from: usize, to: usize, edges: &mut Vec<Vec<usize>>| {
        if from != to && !edges[from].contains(&to) {
            edges[from].push(to);
            indegree[to] += 1;
        }
    };
    for (index, entry) in entries.iter().enumerate() {
        match entry.registration.order {
            Order::Before(id) => {
                if let Some(other) = index_of(id) {
                    add(index, other, &mut edges);
                }
            }
            Order::After(id) => {
                if let Some(other) = index_of(id) {
                    add(other, index, &mut edges);
                }
            }
            _ => {}
        }
    }
    let mut done = vec![false; count];
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let next = (0..count)
            .find(|index| !done[*index] && indegree[*index] == 0)
            // Cycle: take the first remaining entry in base order.
            .or_else(|| (0..count).find(|index| !done[*index]));
        let Some(next) = next else { break };
        done[next] = true;
        for to in &edges[next] {
            indegree[*to] = indegree[*to].saturating_sub(1);
        }
        out.push(entries[next]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    trait Greeter: Send + Sync {
        fn name(&self) -> &'static str;
    }
    struct Named(&'static str);
    impl Greeter for Named {
        fn name(&self) -> &'static str {
            self.0
        }
    }

    fn names(list: Vec<Arc<dyn Greeter>>) -> Vec<&'static str> {
        list.iter().map(|g| g.name()).collect()
    }

    const JS: LanguageId = LanguageId("javascript");
    const TS: LanguageId = LanguageId("typescript");

    fn put(reg: &mut ServiceRegistry, registration: Registration) {
        reg.register::<dyn Greeter>(registration, Arc::new(Named(registration.id)));
    }

    #[test]
    fn get_falls_back_along_the_base_chain_then_any() {
        let mut reg = ServiceRegistry::new();
        reg.declare_base(TS, JS);
        put(&mut reg, Registration::any("generic"));
        assert_eq!(
            reg.get::<dyn Greeter>(TS).map(|g| g.name()),
            Some("generic")
        );
        put(&mut reg, Registration::new("js", JS));
        assert_eq!(reg.get::<dyn Greeter>(TS).map(|g| g.name()), Some("js"));
        put(&mut reg, Registration::new("ts", TS));
        assert_eq!(reg.get::<dyn Greeter>(TS).map(|g| g.name()), Some("ts"));
        assert_eq!(reg.get::<dyn Greeter>(JS).map(|g| g.name()), Some("js"));
        assert_eq!(
            reg.get::<dyn Greeter>(LanguageId("go")).map(|g| g.name()),
            Some("generic")
        );
        assert_eq!(reg.language_chain(TS), vec![TS, JS, LanguageId::ANY]);
    }

    #[test]
    fn all_is_ordered_by_constraints_then_specificity_then_registration() {
        let mut reg = ServiceRegistry::new();
        reg.declare_base(TS, JS);
        put(&mut reg, Registration::any("lsp").order(Order::Last));
        put(&mut reg, Registration::any("generic"));
        put(&mut reg, Registration::new("js", JS));
        put(&mut reg, Registration::new("ts-2", TS));
        put(
            &mut reg,
            Registration::new("ts-1", TS).order(Order::Before("ts-2")),
        );
        put(
            &mut reg,
            Registration::new("templates", TS).order(Order::First),
        );
        put(
            &mut reg,
            Registration::any("after-templates").order(Order::After("templates")),
        );
        assert_eq!(
            names(reg.all::<dyn Greeter>(TS)),
            [
                "templates",
                "ts-1",
                "ts-2",
                "js",
                "generic",
                "after-templates",
                "lsp"
            ]
        );
        assert_eq!(
            names(reg.all::<dyn Greeter>(JS)),
            ["js", "generic", "after-templates", "lsp"]
        );
    }

    #[test]
    fn specific_registration_overrides_the_same_id_and_dumb_mode_filters() {
        let mut reg = ServiceRegistry::new();
        put(&mut reg, Registration::any("fold").dumb_aware(true));
        put(&mut reg, Registration::new("fold", JS));
        put(&mut reg, Registration::any("index-backed"));
        assert_eq!(names(reg.all::<dyn Greeter>(JS)), ["fold", "index-backed"]);
        assert_eq!(
            names(reg.all_available::<dyn Greeter>(JS, true)),
            Vec::<&str>::new()
        );
        assert_eq!(
            names(reg.all_available::<dyn Greeter>(LanguageId("go"), true)),
            ["fold"]
        );
    }

    #[test]
    fn cycles_and_unknown_ids_keep_registration_order() {
        let mut reg = ServiceRegistry::new();
        put(&mut reg, Registration::any("a").order(Order::After("b")));
        put(&mut reg, Registration::any("b").order(Order::After("a")));
        put(
            &mut reg,
            Registration::any("c").order(Order::Before("missing")),
        );
        assert_eq!(names(reg.all::<dyn Greeter>(JS)).len(), 3);
        let mut reg = ServiceRegistry::new();
        reg.declare_base(TS, JS);
        reg.declare_base(JS, TS);
        assert_eq!(reg.language_chain(TS), vec![TS, JS, LanguageId::ANY]);
        assert!(reg.get::<dyn Greeter>(TS).is_none());
        assert_eq!(reg.count::<dyn Greeter>(), 0);
    }
}
