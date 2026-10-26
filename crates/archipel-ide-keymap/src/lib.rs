//! Keymap engine.
//!
//! Loads the 26 keymaps bundled with the reference IDE from
//! `docs/spec/data/keymaps.json` (Apache-2.0 derived data, see
//! `docs/spec/data/NOTICE.md`). Each keymap in the data already contains every
//! effective shortcut (inheritance, the macOS Ctrl/Cmd swap, plugin shortcuts and
//! `use-shortcut-of` are resolved by the generator), so a keymap here is a map from
//! IntelliJ action id to shortcuts. User changes are stored as overrides on top of a
//! bundled keymap. [`Keymap::lookup`] resolves keystrokes (including two-stroke chords)
//! to action ids, and [`Keymap::conflicts`] lists shortcuts bound to several actions.

mod shortcut;

pub use shortcut::{
    KeyStroke, Modifier, Modifiers, MouseButton, Notation, ParseError, Shortcut, canonical_key,
};

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::LazyLock;
use thiserror::Error;

const KEYMAPS_JSON: &str = include_str!("../../../docs/spec/data/keymaps.json");

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KeymapError {
    #[error("unknown keymap `{0}`")]
    UnknownKeymap(String),
    #[error("bundled keymap data is invalid: {0}")]
    Data(String),
    #[error(transparent)]
    Parse(#[from] ParseError),
}

/// Display names of the bundled keymaps (the Settings combo box shows these).
pub fn display_name(id: &str) -> &str {
    match id {
        "$default" => "Windows",
        "Mac OS X 10.5+" => "macOS",
        "Default for GNOME" => "GNOME",
        "Default for KDE" => "KDE",
        "Default for XWin" => "XWin",
        other => other,
    }
}

/// The five primary keymaps, in the order of `docs/spec/keymaps.md`.
pub const PRIMARY_KEYMAPS: [&str; 5] = [
    "$default",
    "Mac OS X 10.5+",
    "Default for GNOME",
    "Default for KDE",
    "Default for XWin",
];

/// Host desktop, used to pick the default keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Desktop {
    MacOs,
    Windows,
    Gnome,
    Kde,
    OtherUnix,
}

impl Desktop {
    /// Detects the running desktop (`XDG_CURRENT_DESKTOP` on Linux/BSD).
    pub fn detect() -> Self {
        if cfg!(target_os = "macos") {
            return Desktop::MacOs;
        }
        if cfg!(windows) {
            return Desktop::Windows;
        }
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .or_else(|_| std::env::var("DESKTOP_SESSION"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        Self::from_desktop_name(&desktop)
    }

    pub fn from_desktop_name(name: &str) -> Self {
        let name = name.to_ascii_lowercase();
        if name.contains("gnome") || name.contains("unity") || name.contains("ubuntu") {
            Desktop::Gnome
        } else if name.contains("kde") || name.contains("plasma") {
            Desktop::Kde
        } else {
            Desktop::OtherUnix
        }
    }

    /// The keymap the reference IDE selects by default on this desktop.
    pub fn default_keymap(self) -> &'static str {
        match self {
            Desktop::MacOs => "Mac OS X 10.5+",
            Desktop::Windows => "$default",
            Desktop::Gnome => "Default for GNOME",
            Desktop::Kde => "Default for KDE",
            Desktop::OtherUnix => "Default for XWin",
        }
    }
}

#[derive(Deserialize)]
struct KeymapsFile {
    keymaps: BTreeMap<String, KeymapData>,
}

#[derive(Deserialize)]
struct KeymapData {
    parent: Option<String>,
    shortcuts: BTreeMap<String, Vec<String>>,
}

/// One bundled keymap as shipped.
#[derive(Debug, Clone)]
pub struct BundledKeymap {
    pub id: String,
    pub parent: Option<String>,
    pub bindings: BTreeMap<String, Vec<Shortcut>>,
}

/// All bundled keymaps.
#[derive(Debug, Clone)]
pub struct KeymapSet {
    keymaps: BTreeMap<String, BundledKeymap>,
}

impl KeymapSet {
    /// Parses the bundled data. Every shortcut must parse.
    pub fn bundled() -> Result<Self, KeymapError> {
        let file: KeymapsFile =
            serde_json::from_str(KEYMAPS_JSON).map_err(|e| KeymapError::Data(e.to_string()))?;
        let mut keymaps = BTreeMap::new();
        for (id, data) in file.keymaps {
            let mut bindings = BTreeMap::new();
            for (action, shortcuts) in data.shortcuts {
                let parsed = shortcuts
                    .iter()
                    .map(|text| text.parse::<Shortcut>())
                    .collect::<Result<Vec<_>, _>>()?;
                bindings.insert(action, parsed);
            }
            keymaps.insert(
                id.clone(),
                BundledKeymap {
                    id,
                    parent: data.parent,
                    bindings,
                },
            );
        }
        Ok(Self { keymaps })
    }

    /// Shared parsed copy of the bundled data (parsed once per process).
    pub fn shared() -> Result<&'static KeymapSet, KeymapError> {
        static SET: LazyLock<Result<KeymapSet, KeymapError>> = LazyLock::new(KeymapSet::bundled);
        SET.as_ref().map_err(Clone::clone)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.keymaps.keys().map(String::as_str)
    }

    pub fn get(&self, id: &str) -> Option<&BundledKeymap> {
        self.keymaps.get(id)
    }

    /// Builds the effective keymap: bundled `id` plus user `overrides`.
    pub fn keymap(&self, id: &str, overrides: &Overrides) -> Result<Keymap, KeymapError> {
        let base = self
            .keymaps
            .get(id)
            .ok_or_else(|| KeymapError::UnknownKeymap(id.into()))?;
        let mut bindings = base.bindings.clone();
        for (action, shortcuts) in &overrides.actions {
            if shortcuts.is_empty() {
                bindings.remove(action);
            } else {
                bindings.insert(action.clone(), shortcuts.clone());
            }
        }
        Ok(Keymap::new(id, bindings))
    }
}

/// User changes on top of a bundled keymap: the full shortcut list per changed action
/// (an empty list removes all shortcuts of that action).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    pub actions: BTreeMap<String, Vec<Shortcut>>,
}

impl Overrides {
    /// Serialized form, one line per action: `ActionId<TAB>shortcut<TAB>shortcut…`.
    pub fn to_lines(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|(action, shortcuts)| {
                let mut line = action.clone();
                for shortcut in shortcuts {
                    line.push('\t');
                    line.push_str(&shortcut.to_string());
                }
                line
            })
            .collect()
    }

    pub fn from_lines(lines: &[String]) -> Result<Self, KeymapError> {
        let mut actions = BTreeMap::new();
        for line in lines {
            let mut parts = line.split('\t');
            let Some(action) = parts.next().filter(|a| !a.trim().is_empty()) else {
                continue;
            };
            let shortcuts = parts
                .map(str::parse::<Shortcut>)
                .collect::<Result<Vec<_>, _>>()?;
            actions.insert(action.trim().to_owned(), shortcuts);
        }
        Ok(Self { actions })
    }
}

/// Result of feeding keystrokes to [`Keymap::lookup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// Actions bound to exactly these strokes, in keymap order.
    Actions(Vec<String>),
    /// The strokes start a chord; wait for the next stroke.
    Prefix,
    None,
}

/// An effective keymap with indexes for lookup and conflict detection.
#[derive(Debug, Clone)]
pub struct Keymap {
    pub id: String,
    bindings: BTreeMap<String, Vec<Shortcut>>,
    by_shortcut: HashMap<Shortcut, Vec<String>>,
    prefixes: BTreeSet<KeyStroke>,
}

impl Keymap {
    pub fn new(id: &str, bindings: BTreeMap<String, Vec<Shortcut>>) -> Self {
        let mut by_shortcut: HashMap<Shortcut, Vec<String>> = HashMap::new();
        let mut prefixes = BTreeSet::new();
        for (action, shortcuts) in &bindings {
            for shortcut in shortcuts {
                by_shortcut
                    .entry(shortcut.clone())
                    .or_default()
                    .push(action.clone());
                if let Shortcut::Keys(strokes) = shortcut
                    && strokes.len() > 1
                {
                    prefixes.insert(strokes[0].clone());
                }
            }
        }
        Self {
            id: id.into(),
            bindings,
            by_shortcut,
            prefixes,
        }
    }

    pub fn shortcuts(&self, action: &str) -> &[Shortcut] {
        self.bindings.get(action).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn bindings(&self) -> &BTreeMap<String, Vec<Shortcut>> {
        &self.bindings
    }

    /// Actions bound to `shortcut`.
    pub fn actions_for(&self, shortcut: &Shortcut) -> &[String] {
        self.by_shortcut
            .get(shortcut)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Resolves typed strokes. A stroke that is both a complete shortcut and a chord
    /// prefix resolves to its actions (as the reference IDE does, the single-stroke
    /// binding wins unless nothing is bound to it).
    pub fn lookup(&self, strokes: &[KeyStroke]) -> Lookup {
        let shortcut = Shortcut::Keys(strokes.to_vec());
        if let Some(actions) = self.by_shortcut.get(&shortcut) {
            return Lookup::Actions(actions.clone());
        }
        if strokes.len() == 1 && self.prefixes.contains(&strokes[0]) {
            return Lookup::Prefix;
        }
        Lookup::None
    }

    /// Shortcuts bound to more than one action, with those actions.
    pub fn conflicts(&self) -> BTreeMap<Shortcut, Vec<String>> {
        self.by_shortcut
            .iter()
            .filter(|(_, actions)| actions.len() > 1)
            .map(|(shortcut, actions)| (shortcut.clone(), actions.clone()))
            .collect()
    }

    /// Other actions that already use `shortcut` (shown by the conflict dialog), plus
    /// actions whose chord starts with it or that it would prefix.
    pub fn conflicts_with(&self, action: &str, shortcut: &Shortcut) -> Vec<String> {
        let mut others: BTreeSet<String> = self
            .actions_for(shortcut)
            .iter()
            .filter(|other| other.as_str() != action)
            .cloned()
            .collect();
        if let Shortcut::Keys(strokes) = shortcut {
            for (other, shortcuts) in &self.bindings {
                if other == action {
                    continue;
                }
                for existing in shortcuts {
                    if let Shortcut::Keys(existing) = existing
                        && existing.len() != strokes.len()
                        && existing[0] == strokes[0]
                    {
                        others.insert(other.clone());
                    }
                }
            }
        }
        others.into_iter().collect()
    }

    /// Actions with a shortcut whose label contains `query` (search by shortcut).
    pub fn find_by_shortcut_text(&self, query: &str, notation: Notation) -> Vec<&str> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        self.bindings
            .iter()
            .filter(|(_, shortcuts)| {
                shortcuts.iter().any(|shortcut| {
                    shortcut.label(notation).to_lowercase().contains(&query)
                        || shortcut.to_string().to_lowercase().contains(&query)
                })
            })
            .map(|(action, _)| action.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests;
