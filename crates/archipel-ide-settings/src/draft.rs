//! Pending edits of an open Settings dialog.

use crate::{SettingValue, SettingsError, SettingsRegistry, SettingsStore};
use std::collections::{BTreeMap, BTreeSet};

/// Edits made in the Settings dialog that are not applied yet.
///
/// Mirrors the dialog's semantics: controls show [`SettingsDraft::value`], editing a
/// value back to the stored one clears the edit, Apply is enabled while
/// [`SettingsDraft::is_modified`], the tree marks [`SettingsDraft::modified_pages`],
/// a page's Reset link calls [`SettingsDraft::reset_page`], OK applies and closes, and
/// Cancel drops the draft.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SettingsDraft {
    changes: BTreeMap<String, SettingValue>,
}

impl SettingsDraft {
    /// The value a control should show: the pending edit, else the stored value.
    pub fn value(
        &self,
        registry: &SettingsRegistry,
        store: &SettingsStore,
        id: &str,
    ) -> Result<SettingValue, SettingsError> {
        match self.changes.get(id) {
            Some(value) => Ok(value.clone()),
            None => store.get(registry, id),
        }
    }

    /// Records an edit after validating it.
    pub fn set(
        &mut self,
        registry: &SettingsRegistry,
        store: &SettingsStore,
        id: &str,
        value: SettingValue,
    ) -> Result<(), SettingsError> {
        let def = registry
            .setting(id)
            .ok_or_else(|| SettingsError::UnknownSetting(id.into()))?;
        let value = def.validate(&value)?;
        if store.get(registry, id)? == value {
            self.changes.remove(id);
        } else {
            self.changes.insert(id.into(), value);
        }
        Ok(())
    }

    pub fn is_modified(&self) -> bool {
        !self.changes.is_empty()
    }

    pub fn is_setting_modified(&self, id: &str) -> bool {
        self.changes.contains_key(id)
    }

    pub fn changes(&self) -> &BTreeMap<String, SettingValue> {
        &self.changes
    }

    /// Pages with pending edits and their ancestors (both are marked in the tree).
    pub fn modified_pages(&self, registry: &SettingsRegistry) -> BTreeSet<String> {
        let mut pages = BTreeSet::new();
        for id in self.changes.keys() {
            let mut current = registry.setting(id).map(|def| def.page.as_str());
            while let Some(page) = current.and_then(|id| registry.page(id)) {
                if !pages.insert(page.id.clone()) {
                    break;
                }
                current = page.parent.as_deref();
            }
        }
        pages
    }

    /// Drops the pending edits of one page (the page's "Reset" link).
    pub fn reset_page(&mut self, registry: &SettingsRegistry, page: &str) {
        self.changes
            .retain(|id, _| registry.setting(id).is_none_or(|def| def.page != page));
    }

    /// Edits every setting of `page` back to its default (still pending until Apply).
    pub fn restore_defaults(
        &mut self,
        registry: &SettingsRegistry,
        store: &SettingsStore,
        page: &str,
    ) -> Result<(), SettingsError> {
        for def in registry.page_settings(page) {
            self.set(registry, store, &def.id, def.default.clone())?;
        }
        Ok(())
    }

    /// Writes all edits into `store` (all-or-nothing) and clears the draft. Returns the
    /// ids that changed.
    pub fn apply(
        &mut self,
        registry: &SettingsRegistry,
        store: &mut SettingsStore,
    ) -> Result<Vec<String>, SettingsError> {
        let mut next = store.clone();
        for (id, value) in &self.changes {
            next.set(registry, id, value.clone())?;
        }
        *store = next;
        let changed = std::mem::take(&mut self.changes).into_keys().collect();
        Ok(changed)
    }
}
