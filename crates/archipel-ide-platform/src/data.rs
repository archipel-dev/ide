//! `DataContext`: what the UI and actions know about "where the user is right now".
//!
//! In the IntelliJ Platform a `DataContext` answers `getData(key)` for the focused component and
//! its ancestors. A menu item, a toolbar button or a keyboard shortcut asks the context for the
//! project, the file in the editor, the selection and so on, instead of reaching into the
//! feature that owns them. Here the same idea has three parts:
//!
//! - [`DataKey<T>`]: a typed name for one piece of data (`keys::ACTIVE_FILE`).
//! - [`DataProvider`]: something that can answer for keys (the IDE state, the focused widget).
//!   Providers answer lazily; a key nobody asked for costs nothing.
//! - [`DataContext`]: an ordered set of providers and explicit values. The innermost source
//!   (the one added last, like the focused component) answers first.
//!
//! A context borrows its providers (`DataContext<'a>`), so building one per frame copies nothing.
//! Values come out as owned clones (`T: Clone`), which keeps a provider free to compute them on
//! demand without handing out references into its own state.

use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

/// A typed name for a piece of data. Two keys with the same name and type are the same key.
pub struct DataKey<T: 'static> {
    name: Cow<'static, str>,
    marker: PhantomData<fn() -> T>,
}

impl<T: 'static> DataKey<T> {
    /// A key with a fixed name, usable in a `const`.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name: Cow::Borrowed(name),
            marker: PhantomData,
        }
    }

    /// A key whose name is computed (for example one flag per command id).
    pub fn dynamic(name: impl Into<String>) -> Self {
        Self {
            name: Cow::Owned(name.into()),
            marker: PhantomData,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T: 'static> Clone for DataKey<T> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            marker: PhantomData,
        }
    }
}

impl<T: 'static> fmt::Debug for DataKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DataKey({})", self.name)
    }
}

/// A source of data. Answers `None` for a key it does not know, so the next source can.
pub trait DataProvider {
    /// The value for `key`, boxed as the key's type. The caller downcasts; a value of another
    /// type counts as missing.
    fn data(&self, key: &str) -> Option<Box<dyn Any>>;
}

/// An ordered set of data sources. See the module documentation.
#[derive(Default)]
pub struct DataContext<'a> {
    providers: Vec<&'a dyn DataProvider>,
    values: HashMap<String, Box<dyn Any>>,
}

impl<'a> DataContext<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a source that answers before the ones added earlier (the focused component asks
    /// before its parents).
    #[must_use]
    pub fn with_provider(mut self, provider: &'a dyn DataProvider) -> Self {
        self.providers.push(provider);
        self
    }

    /// Fixes `key` to `value`, ahead of every provider.
    #[must_use]
    pub fn with_value<T: 'static>(mut self, key: &DataKey<T>, value: T) -> Self {
        self.values.insert(key.name().to_owned(), Box::new(value));
        self
    }

    /// The value of `key`, if some source has one of the right type.
    pub fn get<T: Clone + 'static>(&self, key: &DataKey<T>) -> Option<T> {
        if let Some(value) = self.values.get(key.name()) {
            return value.downcast_ref::<T>().cloned();
        }
        self.providers
            .iter()
            .rev()
            .find_map(|provider| provider.data(key.name()))
            .and_then(|value| value.downcast::<T>().ok())
            .map(|value| *value)
    }

    /// Whether `key` has a value (a flag key that is `false` still has one).
    pub fn has<T: Clone + 'static>(&self, key: &DataKey<T>) -> bool {
        self.get(key).is_some()
    }

    /// A boolean fact; missing counts as `false`.
    pub fn flag(&self, key: &DataKey<bool>) -> bool {
        self.get(key).unwrap_or(false)
    }
}

impl fmt::Debug for DataContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataContext")
            .field("providers", &self.providers.len())
            .field("values", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// A caret position, zero-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Caret {
    pub line: usize,
    pub column: usize,
}

/// The keys every provider of the IDE agrees on (IntelliJ: `CommonDataKeys`, `PlatformDataKeys`).
/// A provider answers only for what is true right now: no project means no `PROJECT_ROOT`.
pub mod keys {
    use super::{Caret, DataKey};
    use std::path::PathBuf;

    /// Root directory of the open project.
    pub const PROJECT_ROOT: DataKey<PathBuf> = DataKey::new("project.root");
    /// The file in the selected editor tab, project-relative. Missing when no editor is open
    /// or the diff tab is showing a file-less comparison.
    pub const ACTIVE_FILE: DataKey<PathBuf> = DataKey::new("editor.file");
    /// Number of open editor tabs.
    pub const OPEN_EDITORS: DataKey<usize> = DataKey::new("editor.count");
    /// Whether any open editor has unsaved changes.
    pub const HAS_UNSAVED: DataKey<bool> = DataKey::new("editor.unsaved");
    /// Caret of the active editor.
    pub const CARET: DataKey<Caret> = DataKey::new("editor.caret");
    /// The selected text of the active editor; missing when nothing is selected.
    pub const SELECTED_TEXT: DataKey<String> = DataKey::new("editor.selection");
    /// Whether the text editor has keyboard focus (not a text field or a tool window).
    pub const EDITOR_FOCUSED: DataKey<bool> = DataKey::new("editor.focused");
    /// Whether the diff viewer tab is the selected editor tab.
    pub const DIFF_ACTIVE: DataKey<bool> = DataKey::new("editor.diffActive");
    /// Whether the open project is a Git working tree.
    pub const VCS_ACTIVE: DataKey<bool> = DataKey::new("vcs.active");
    /// Whether a debug session is running.
    pub const DEBUG_SESSION: DataKey<bool> = DataKey::new("debug.session");
    /// Whether the open project is an Android project.
    pub const ANDROID_PROJECT: DataKey<bool> = DataKey::new("android.project");
    /// The text of the Find bar, when it has one.
    pub const FIND_QUERY: DataKey<String> = DataKey::new("find.query");
    /// The entry selected in the Project tool window, project-relative.
    pub const TREE_SELECTION: DataKey<PathBuf> = DataKey::new("project.treeSelection");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct Fixed(&'static [(&'static str, i32)]);

    impl DataProvider for Fixed {
        fn data(&self, key: &str) -> Option<Box<dyn Any>> {
            self.0
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| Box::new(*value) as Box<dyn Any>)
        }
    }

    const NUMBER: DataKey<i32> = DataKey::new("n");
    const OTHER: DataKey<i32> = DataKey::new("o");

    #[test]
    fn the_last_provider_answers_first_and_the_others_fill_in() {
        let outer = Fixed(&[("n", 1), ("o", 10)]);
        let inner = Fixed(&[("n", 2)]);
        let context = DataContext::new()
            .with_provider(&outer)
            .with_provider(&inner);
        assert_eq!(context.get(&NUMBER), Some(2));
        assert_eq!(context.get(&OTHER), Some(10));
        assert_eq!(context.get(&DataKey::<i32>::new("missing")), None);
    }

    #[test]
    fn explicit_values_beat_providers() {
        let provider = Fixed(&[("n", 1)]);
        let context = DataContext::new()
            .with_provider(&provider)
            .with_value(&NUMBER, 5);
        assert_eq!(context.get(&NUMBER), Some(5));
    }

    #[test]
    fn a_value_of_the_wrong_type_is_missing() {
        let provider = Fixed(&[("n", 1)]);
        let context = DataContext::new().with_provider(&provider);
        assert_eq!(context.get(&DataKey::<String>::new("n")), None);
    }

    #[test]
    fn flags_default_to_false_and_dynamic_keys_match_by_name() {
        struct Flags;
        impl DataProvider for Flags {
            fn data(&self, key: &str) -> Option<Box<dyn Any>> {
                key.strip_prefix("flag:")
                    .map(|rest| Box::new(rest == "on") as Box<dyn Any>)
            }
        }
        let context = DataContext::new().with_provider(&Flags);
        assert!(context.flag(&DataKey::dynamic("flag:on")));
        assert!(!context.flag(&DataKey::dynamic("flag:off")));
        assert!(!context.flag(&DataKey::new("unrelated")));
        assert!(context.has(&DataKey::<bool>::dynamic("flag:off")));
    }

    #[test]
    fn providers_are_asked_lazily() {
        struct Counting(Cell<usize>);
        impl DataProvider for Counting {
            fn data(&self, _: &str) -> Option<Box<dyn Any>> {
                self.0.set(self.0.get() + 1);
                None
            }
        }
        let counting = Counting(Cell::new(0));
        let context = DataContext::new().with_provider(&counting);
        assert_eq!(counting.0.get(), 0, "building a context asks nothing");
        let _ = context.get(&NUMBER);
        assert_eq!(counting.0.get(), 1);
    }
}
