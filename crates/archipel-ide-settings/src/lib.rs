//! Typed, UI-independent settings model.
//!
//! The model mirrors the observable structure of the reference IDE's Settings dialog:
//! a tree of pages keyed by the configurable ids in `docs/spec/data/settings.json`,
//! each page holding typed settings with defaults and validation. Values are layered
//! (default → application → project), persisted as TOML, and edited through a
//! [`SettingsDraft`] that implements the dialog's Apply / Reset / OK / Cancel semantics.
//!
//! Other subsystems contribute pages and settings with [`SettingsRegistry::register_page`]
//! and [`SettingsRegistry::register_setting`]; see `docs/architecture.md`.

mod draft;
mod intellij;
mod store;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub use draft::SettingsDraft;
pub use intellij::{ADVANCED_SETTINGS_PAGE, AGENTS_PAGE, TOP_LEVEL_ORDER, ids};
pub use store::{SettingsStore, default_config_dir};

/// Where a setting (or page) is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Per user, shared by all projects.
    Application,
    /// Per project (`.archipel/settings.toml`); falls back to the application layer.
    Project,
}

/// A typed setting value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    List(Vec<String>),
}

impl SettingValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SettingValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            SettingValue::Int(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            SettingValue::Float(value) => Some(*value),
            SettingValue::Int(value) => Some(*value as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            SettingValue::Text(value) => Some(value),
            _ => None,
        }
    }

    /// Converts JSON (command parameters) into a value without type checking.
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(value.clone()).ok()
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// The type of a setting and its constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SettingKind {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Float {
        min: f64,
        max: f64,
    },
    Text,
    /// A file system path; empty means "not set / detect".
    Path,
    /// One of the listed option ids.
    Enum {
        options: Vec<EnumOption>,
    },
    List,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumOption {
    pub id: String,
    pub label: String,
}

impl EnumOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Definition of one setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingDef {
    /// Stable id, also the TOML key.
    pub id: String,
    /// Page (configurable id) that shows this setting.
    pub page: String,
    /// Group heading on the page (e.g. "Accessibility"); `None` for the top block.
    pub group: Option<String>,
    pub label: String,
    pub description: Option<String>,
    pub kind: SettingKind,
    pub default: SettingValue,
    pub scope: Scope,
    /// Changing the value only takes effect after a restart.
    pub requires_restart: bool,
}

impl SettingDef {
    pub fn new(
        id: impl Into<String>,
        page: impl Into<String>,
        label: impl Into<String>,
        kind: SettingKind,
        default: SettingValue,
    ) -> Self {
        Self {
            id: id.into(),
            page: page.into(),
            group: None,
            label: label.into(),
            description: None,
            kind,
            default,
            scope: Scope::Application,
            requires_restart: false,
        }
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn restart(mut self) -> Self {
        self.requires_restart = true;
        self
    }

    /// Checks that `value` has this setting's type and satisfies its constraints.
    pub fn validate(&self, value: &SettingValue) -> Result<SettingValue, SettingsError> {
        let invalid = |reason: String| SettingsError::InvalidValue {
            id: self.id.clone(),
            reason,
        };
        match (&self.kind, value) {
            (SettingKind::Bool, SettingValue::Bool(_)) => Ok(value.clone()),
            (SettingKind::Int { min, max }, SettingValue::Int(number)) => {
                if number < min || number > max {
                    Err(invalid(format!("must be between {min} and {max}")))
                } else {
                    Ok(value.clone())
                }
            }
            (SettingKind::Float { min, max }, value) => match value.as_float() {
                Some(number) if number.is_finite() && number >= *min && number <= *max => {
                    Ok(SettingValue::Float(number))
                }
                Some(_) => Err(invalid(format!("must be between {min} and {max}"))),
                None => Err(invalid("expected a number".into())),
            },
            (SettingKind::Text | SettingKind::Path, SettingValue::Text(text)) => {
                if text.contains('\0') {
                    Err(invalid("must not contain NUL characters".into()))
                } else {
                    Ok(value.clone())
                }
            }
            (SettingKind::Enum { options }, SettingValue::Text(id)) => {
                if options.iter().any(|option| &option.id == id) {
                    Ok(value.clone())
                } else {
                    let allowed: Vec<_> = options.iter().map(|o| o.id.as_str()).collect();
                    Err(invalid(format!("expected one of {}", allowed.join(", "))))
                }
            }
            (SettingKind::List, SettingValue::List(_)) => Ok(value.clone()),
            (kind, _) => Err(invalid(format!("expected {}", kind_name(kind)))),
        }
    }
}

fn kind_name(kind: &SettingKind) -> &'static str {
    match kind {
        SettingKind::Bool => "a boolean",
        SettingKind::Int { .. } => "an integer",
        SettingKind::Float { .. } => "a number",
        SettingKind::Text => "a string",
        SettingKind::Path => "a path string",
        SettingKind::Enum { .. } => "an option id string",
        SettingKind::List => "a list of strings",
    }
}

/// One node of the Settings tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageDef {
    /// Configurable id (e.g. `preferences.lookFeel`) or group id (e.g. `appearance`).
    pub id: String,
    pub parent: Option<String>,
    pub label: String,
    pub level: Scope,
    /// Higher weights sort first among siblings, then by label.
    pub weight: i32,
}

impl PageDef {
    pub fn new(id: impl Into<String>, parent: Option<&str>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            parent: parent.map(Into::into),
            label: label.into(),
            level: Scope::Application,
            weight: 0,
        }
    }

    pub fn level(mut self, level: Scope) -> Self {
        self.level = level;
        self
    }

    pub fn weight(mut self, weight: i32) -> Self {
        self.weight = weight;
        self
    }
}

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettingsError {
    #[error("unknown setting `{0}`")]
    UnknownSetting(String),
    #[error("unknown settings page `{0}`")]
    UnknownPage(String),
    #[error("duplicate {kind} id `{id}`")]
    Duplicate { kind: String, id: String },
    #[error("invalid value for `{id}`: {reason}")]
    InvalidValue { id: String, reason: String },
    #[error("could not read settings file {path}: {reason}")]
    Read { path: String, reason: String },
    #[error("could not write settings file {path}: {reason}")]
    Write { path: String, reason: String },
}

/// Pages that match a search, plus the matching settings inside them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchResult {
    /// Pages whose label or settings match.
    pub matching_pages: BTreeSet<String>,
    /// Matching pages and all their ancestors (what the filtered tree shows).
    pub visible_pages: BTreeSet<String>,
    pub matching_settings: BTreeSet<String>,
}

/// All known pages and settings.
#[derive(Debug, Clone, Default)]
pub struct SettingsRegistry {
    pages: BTreeMap<String, PageDef>,
    settings: BTreeMap<String, SettingDef>,
    /// Setting ids per page in registration order (the order the page shows them).
    page_settings: BTreeMap<String, Vec<String>>,
}

impl SettingsRegistry {
    /// The reference IDE's Settings tree with Archipel IDE's initial setting definitions.
    pub fn intellij() -> Self {
        intellij::registry()
    }

    pub fn register_page(&mut self, page: PageDef) -> Result<(), SettingsError> {
        if let Some(parent) = &page.parent
            && !self.pages.contains_key(parent)
        {
            return Err(SettingsError::UnknownPage(parent.clone()));
        }
        if self.pages.contains_key(&page.id) {
            return Err(SettingsError::Duplicate {
                kind: "page".into(),
                id: page.id,
            });
        }
        self.pages.insert(page.id.clone(), page);
        Ok(())
    }

    pub fn register_setting(&mut self, setting: SettingDef) -> Result<(), SettingsError> {
        if !self.pages.contains_key(&setting.page) {
            return Err(SettingsError::UnknownPage(setting.page));
        }
        if self.settings.contains_key(&setting.id) {
            return Err(SettingsError::Duplicate {
                kind: "setting".into(),
                id: setting.id,
            });
        }
        setting.validate(&setting.default)?;
        self.page_settings
            .entry(setting.page.clone())
            .or_default()
            .push(setting.id.clone());
        self.settings.insert(setting.id.clone(), setting);
        Ok(())
    }

    pub fn page(&self, id: &str) -> Option<&PageDef> {
        self.pages.get(id)
    }

    pub fn setting(&self, id: &str) -> Option<&SettingDef> {
        self.settings.get(id)
    }

    pub fn settings(&self) -> impl Iterator<Item = &SettingDef> {
        self.settings.values()
    }

    /// Settings shown on `page`, in registration order.
    pub fn page_settings(&self, page: &str) -> Vec<&SettingDef> {
        self.page_settings
            .get(page)
            .into_iter()
            .flatten()
            .filter_map(|id| self.settings.get(id))
            .collect()
    }

    /// Children of `parent` (`None` for the top level), in display order.
    pub fn children(&self, parent: Option<&str>) -> Vec<&PageDef> {
        let mut children: Vec<_> = self
            .pages
            .values()
            .filter(|page| page.parent.as_deref() == parent)
            .collect();
        if parent.is_none() {
            let rank = |id: &str| {
                TOP_LEVEL_ORDER
                    .iter()
                    .position(|top| *top == id)
                    .unwrap_or(TOP_LEVEL_ORDER.len())
            };
            children.sort_by(|a, b| {
                rank(&a.id)
                    .cmp(&rank(&b.id))
                    .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            });
        } else {
            children.sort_by(|a, b| {
                b.weight
                    .cmp(&a.weight)
                    .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
            });
        }
        children
    }

    /// Labels from the top-level group down to `page`, e.g.
    /// `["Appearance & Behavior", "Appearance"]`.
    pub fn breadcrumbs(&self, page: &str) -> Vec<&str> {
        let mut crumbs = Vec::new();
        let mut current = self.pages.get(page);
        while let Some(node) = current {
            crumbs.push(node.label.as_str());
            current = node.parent.as_deref().and_then(|id| self.pages.get(id));
            if crumbs.len() > self.pages.len() {
                break;
            }
        }
        crumbs.reverse();
        crumbs
    }

    /// Case-insensitive search over page labels and setting labels, descriptions,
    /// groups, enum option labels and the page label. Every whitespace-separated term must match.
    pub fn search(&self, query: &str) -> SearchResult {
        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let mut result = SearchResult::default();
        if terms.is_empty() {
            result.visible_pages = self.pages.keys().cloned().collect();
            return result;
        }
        let matches = |haystack: &str| terms.iter().all(|term| haystack.contains(term.as_str()));
        for setting in self.settings.values() {
            let mut text = setting.label.to_lowercase();
            // The page name qualifies its settings ("font size" finds Font > Size).
            if let Some(page) = self.pages.get(&setting.page) {
                text.push(' ');
                text.push_str(&page.label.to_lowercase());
            }
            for extra in [&setting.description, &setting.group].into_iter().flatten() {
                text.push(' ');
                text.push_str(&extra.to_lowercase());
            }
            if let SettingKind::Enum { options } = &setting.kind {
                for option in options {
                    text.push(' ');
                    text.push_str(&option.label.to_lowercase());
                }
            }
            if matches(&text) {
                result.matching_settings.insert(setting.id.clone());
                result.matching_pages.insert(setting.page.clone());
            }
        }
        for page in self.pages.values() {
            if matches(&page.label.to_lowercase()) {
                result.matching_pages.insert(page.id.clone());
            }
        }
        for id in &result.matching_pages {
            let mut current = Some(id.as_str());
            while let Some(page) = current.and_then(|id| self.pages.get(id)) {
                if !result.visible_pages.insert(page.id.clone()) {
                    break;
                }
                current = page.parent.as_deref();
            }
        }
        result
    }
}

#[cfg(test)]
mod tests;
