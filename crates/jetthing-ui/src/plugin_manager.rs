//! Settings › Plugins page (observable layout of the reference scene
//! `24-plugins`), registered as the `preferences.pluginManager` settings page:
//! Marketplace / Installed tabs with a gear menu, a searchable grouped plugin list
//! (downloaded, project and bundled plugins from the plugin registry, plus manifests
//! that failed to load), and a details pane with Enable/Disable, Uninstall and
//! Overview / Settings / Additional Info tabs. The page reads the IDE state and
//! queues commands (`plugin.enable`, `plugin.disable`, `plugin.install`,
//! `plugin.uninstall`, `plugin.refresh`, `plugin.setSetting`) that the settings
//! dialog runs after drawing.

use crate::dialog;
use crate::widgets;
use archipel_kit::components::{
    Avatar, Banner, Button, Checkbox, Dropdown, EmptyState, IconButton, Link, ListRow, MenuItem,
    PropertyGrid, SectionHeader, Severity, Tab as KitTab, TabBar, TextField, VirtualList,
};
use archipel_kit::parts::{Popover, Slot};
use archipel_kit::tokens::typography;
use archipel_kit::{Glyph, Theme, Tone};
use eframe::egui::{self, RichText, vec2};
use jetthing_core::IdeState;
use jetthing_plugins::catalog::{Catalog, CatalogEntry};
use jetthing_plugins::{PluginEntry, PluginSource, PluginStatus, SettingKind};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    Marketplace,
    #[default]
    Installed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailTab {
    #[default]
    Overview,
    Settings,
    AdditionalInfo,
}

#[derive(Debug, Default)]
pub struct PluginManager {
    pub tab: Tab,
    pub selected: Option<String>,
    pub search: String,
    pub detail: DetailTab,
    /// Path field of "Install Plugin from Disk…" while it is open.
    pub install_path: Option<String>,
    pub install_error: Option<String>,
    /// Local registry listing of the Marketplace tab; read when the tab opens.
    catalog: Option<Catalog>,
    /// Selected Marketplace plugin id.
    market_selected: Option<String>,
}

const LIST_WIDTH: f32 = 340.0;

/// Groups of the Installed tab, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Downloaded,
    Project,
    Bundled,
}

fn group_of(state: &IdeState, entry: &PluginEntry) -> Group {
    match &entry.source {
        PluginSource::Bundled => Group::Bundled,
        PluginSource::Directory(_) if state.plugin_uninstallable(entry.id()) => Group::Downloaded,
        PluginSource::Directory(dir)
            if state
                .project
                .as_ref()
                .is_some_and(|project| dir.starts_with(project.root())) =>
        {
            Group::Project
        }
        // `JETTHING_PLUGINS_DIR` and `plugins/` next to the executable.
        PluginSource::Directory(_) => Group::Bundled,
    }
}

fn matches(entry: &PluginEntry, query: &str) -> bool {
    matches_manifest(&entry.manifest, query)
}

fn matches_manifest(manifest: &jetthing_plugins::PluginManifest, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    [
        manifest.name.as_str(),
        manifest.id.as_str(),
        manifest.vendor.as_str(),
        manifest.description.as_str(),
    ]
    .iter()
    .chain(
        manifest
            .capabilities
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .iter(),
    )
    .any(|field| field.to_lowercase().contains(&query))
}

fn enabled(entry: &PluginEntry) -> bool {
    !matches!(
        entry.status,
        PluginStatus::Disabled | PluginStatus::Unsupported(_)
    )
}

/// What the page reads, and the commands it queues for the settings dialog.
pub struct PageContext<'a> {
    pub state: &'a IdeState,
    pub requests: &'a mut Vec<(String, Value)>,
}

impl PageContext<'_> {
    fn dispatch(&mut self, id: &str, params: Value) {
        self.requests.push((id.to_owned(), params));
    }
}

impl PluginManager {
    /// Handles the result of a queued `plugin.install` request.
    pub fn install_finished(&mut self, result: Result<Value, String>) {
        match result {
            Ok(result) => {
                self.selected = result["id"].as_str().map(str::to_owned);
                self.install_path = None;
                self.install_error = None;
            }
            Err(error) => self.install_error = Some(error),
        }
    }
}

/// The page body, filling the rest of the settings dialog's page area.
pub fn page(ui: &mut egui::Ui, manager: &mut PluginManager, cx: &mut PageContext<'_>) {
    let registry = &cx.state.plugin_host.registry;
    if manager.tab == Tab::Installed
        && manager
            .selected
            .as_deref()
            .is_none_or(|id| registry.get(id).is_none())
    {
        // Like the reference, the first plugin is previewed until another is picked.
        manager.selected = registry
            .entries()
            .first()
            .map(|entry| entry.id().to_owned());
    }
    if manager.tab == Tab::Marketplace && manager.catalog.is_none() {
        manager.catalog = Some(Catalog::scan(&jetthing_core::plugin_registry_dirs()));
    }
    header_row(cx, ui, manager);
    ui.separator();
    let height = ui.available_height();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.allocate_ui_with_layout(
            vec2(LIST_WIDTH, height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_size(vec2(LIST_WIDTH, height));
                list(cx, ui, manager);
            },
        );
        ui.separator();
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            vec2(width, height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.set_min_size(vec2(width, height));
                ui.add_space(8.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - 12.0);
                        match manager.tab {
                            Tab::Installed => details(cx, ui, manager),
                            Tab::Marketplace => market_details(cx, ui, manager),
                        }
                    });
                });
            },
        );
    });
}

/// Where plugins can be installed from, shown when nothing is selected.
fn plugin_directories(cx: &PageContext<'_>, ui: &mut egui::Ui) {
    ui.add(SectionHeader::new("Plugin directories"));
    dialog::hint(
        ui,
        "Jetthing loads plugins (a folder with a jetthing-plugin.toml) from these directories; \
         .jtplugin archives install into the User directory:",
    );
    let project = cx.state.project.as_ref().map(|p| p.root());
    let mut grid = PropertyGrid::new("plugin_directories");
    for (source, dir) in jetthing_core::plugin_dirs(project) {
        grid = grid.mono_row(source, dir.display().to_string());
    }
    grid.show(ui);
}

fn header_row(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    let installed = cx.state.plugin_host.registry.entries().len();
    let tabs = [
        KitTab::new("Marketplace"),
        KitTab::new(format!("Installed ({installed})")),
    ];
    let current = usize::from(manager.tab == Tab::Installed);
    ui.horizontal(|ui| {
        if let Some(index) = TabBar::new(&tabs, current).show(ui).selected {
            manager.tab = if index == 0 {
                Tab::Marketplace
            } else {
                Tab::Installed
            };
            if manager.tab == Tab::Marketplace {
                manager.catalog = None; // rescan the registries
            }
        }
        let gear = ui.add(IconButton::new(Glyph::Settings, "Manage plugins"));
        let id = egui::Id::new("plugin_manager_gear_open");
        let mut open: bool = ui.data(|d| d.get_temp(id)).unwrap_or(false);
        if gear.clicked() {
            open = !open;
        }
        let mut picked = false;
        Popover::below("plugin_manager_gear", &gear)
            .width(240.0)
            .show(ui.ctx(), &mut open, |ui| {
                if ui.add(MenuItem::new("Install Plugin from Disk…")).clicked() {
                    manager.tab = Tab::Installed;
                    manager.install_path = Some(String::new());
                    manager.install_error = None;
                    picked = true;
                }
                if ui.add(MenuItem::new("Refresh Plugin List")).clicked() {
                    manager.catalog = None;
                    cx.dispatch("plugin.refresh", Value::Null);
                    picked = true;
                }
            });
        ui.data_mut(|d| d.insert_temp(id, open && !picked));
    });
}

fn list(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        widgets::search_field(
            ui,
            egui::Id::new("plugin_manager_search"),
            &mut manager.search,
            "Type / to see options",
            LIST_WIDTH - 60.0,
        );
    });
    ui.add_space(8.0);
    if let Some(path) = manager.install_path.as_mut() {
        let mut install = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.set_width(LIST_WIDTH - 24.0);
                let error = manager
                    .install_error
                    .as_deref()
                    .map(|error| dialog::friendly("Plugin file", error));
                let response = ui.add(
                    TextField::new(path)
                        .label("Install Plugin from Disk")
                        .placeholder("/path/to/plugin.jtplugin")
                        .error(error.as_deref()),
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    install = true;
                }
                ui.horizontal(|ui| {
                    install |= ui.add(Button::primary("Install")).clicked();
                    cancel = ui.add(Button::secondary("Cancel")).clicked();
                });
            });
        });
        ui.add_space(8.0);
        if install && !path.trim().is_empty() {
            let path = path.trim().to_owned();
            cx.dispatch("plugin.install", json!({"path": path}));
        }
        if cancel {
            manager.install_path = None;
            manager.install_error = None;
        }
    }
    match manager.tab {
        Tab::Marketplace => marketplace(cx, ui, manager),
        Tab::Installed => installed_list(cx, ui, manager),
    }
}

/// A row of a plugin list: a group heading (with an optional Enable/Disable all link), a
/// plugin or a load problem.
enum Row<'a> {
    Group {
        title: String,
        /// `Some(enable)` when the link should ask for that state for `ids`.
        link: Option<(bool, Vec<String>)>,
    },
    Installed(&'a PluginEntry),
    Registry {
        entry: &'a CatalogEntry,
        installed: Option<String>,
    },
    Problem(String),
}

impl Row<'_> {
    fn selectable(&self) -> bool {
        matches!(self, Row::Installed(_) | Row::Registry { .. })
    }
}

fn marketplace(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    let catalog = manager.catalog.clone().unwrap_or_default();
    let visible: Vec<&CatalogEntry> = catalog
        .entries
        .iter()
        .filter(|entry| matches_manifest(&entry.manifest, &manager.search))
        .collect();
    if manager.market_selected.is_none() {
        manager.market_selected = visible.first().map(|entry| entry.manifest.id.clone());
    }
    if visible.is_empty() {
        let (title, hint) = if catalog.entries.is_empty() {
            (
                "No plugins in the local registry",
                "Put .jtplugin archives into one of the registry directories listed on the right.",
            )
        } else {
            ("Nothing found", "No registry plugin matches the search.")
        };
        let out = EmptyState::new(title).hint(hint).show(ui);
        let _ = out;
        if ui
            .add(Button::secondary("Install Plugin from Disk…"))
            .clicked()
        {
            manager.tab = Tab::Installed;
            manager.install_path = Some(String::new());
        }
        return;
    }
    let mut rows = vec![Row::Group {
        title: "Local registry".into(),
        link: None,
    }];
    rows.extend(visible.iter().map(|entry| {
        Row::Registry {
            entry,
            installed: cx
                .state
                .plugin_host
                .registry
                .get(&entry.manifest.id)
                .map(|e| e.manifest.version.clone()),
        }
    }));
    let installs = std::cell::RefCell::new(Vec::<PathBuf>::new());
    let toggles = std::cell::RefCell::new(Vec::<(String, bool)>::new());
    let current = manager.market_selected.clone();
    let chosen = plugin_list(
        ui,
        "plugin_market_list",
        "Marketplace plugins",
        &rows,
        |row| matches!(row, Row::Registry { entry, .. } if Some(entry.manifest.id.as_str()) == current.as_deref()),
        &installs,
        &toggles,
    );
    if let Some(Row::Registry { entry, .. }) = chosen.map(|index| &rows[index]) {
        manager.market_selected = Some(entry.manifest.id.clone());
        manager.detail = DetailTab::Overview;
    }
    for archive in installs.into_inner() {
        manager.install_error = None;
        cx.dispatch(
            "plugin.install",
            json!({"path": archive.display().to_string()}),
        );
    }
}

/// Draws `rows` as a virtualized list and returns the row the user selected this frame.
/// Install buttons push archives to `installs`, enable checkboxes and group links push
/// `(id, enable)` to `toggles`.
fn plugin_list(
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    rows: &[Row<'_>],
    is_selected: impl Fn(&Row<'_>) -> bool,
    installs: &std::cell::RefCell<Vec<PathBuf>>,
    toggles: &std::cell::RefCell<Vec<(String, bool)>>,
) -> Option<usize> {
    let from = rows.iter().position(&is_selected);
    let mut selected = from;
    VirtualList::new(id, rows.len(), &mut selected)
        .name(name)
        .height(ui.available_height())
        .show(ui, |index| match &rows[index] {
            Row::Group { title, link } => {
                let row = ListRow::new(title).label_tone(Tone::Secondary).flat(true);
                match link {
                    Some((enable, ids)) => {
                        let label = if *enable { "Enable all" } else { "Disable all" };
                        row.trailing(Slot::ui_width(86.0, move |ui| {
                            if ui.add(Link::new(label)).clicked() {
                                toggles
                                    .borrow_mut()
                                    .extend(ids.iter().map(|id| (id.clone(), *enable)));
                            }
                        }))
                    }
                    None => row,
                }
            }
            Row::Installed(entry) => {
                let manifest = &entry.manifest;
                let tone = match entry.status {
                    PluginStatus::Failed(_) => Tone::Error,
                    _ if !enabled(entry) => Tone::Secondary,
                    _ => Tone::Strong,
                };
                let detail = match &entry.status {
                    PluginStatus::Failed(_) => format!("{}  failed to load", manifest.version),
                    PluginStatus::Unsupported(_) => format!("{}  unsupported", manifest.version),
                    _ if entry.source == PluginSource::Bundled => {
                        format!("{}  bundled", manifest.version)
                    }
                    _ => format!("{}  {}", manifest.version, manifest.vendor),
                };
                let can_toggle = entry.can_toggle();
                let on = enabled(entry);
                let plugin_id = entry.id().to_owned();
                ListRow::new(&manifest.name)
                    .icon(Glyph::Plugin)
                    .label_tone(tone)
                    .secondary(detail)
                    .trailing(Slot::ui_width(96.0, move |ui| {
                        let mut checked = on;
                        if ui
                            .add(Checkbox::new(&mut checked, "Enabled").enabled(can_toggle))
                            .changed()
                        {
                            toggles.borrow_mut().push((plugin_id.clone(), checked));
                        }
                    }))
            }
            Row::Registry { entry, installed } => {
                let manifest = &entry.manifest;
                let vendor = if manifest.vendor.is_empty() {
                    "Unknown vendor"
                } else {
                    &manifest.vendor
                };
                let (label, enabled, primary) = match installed {
                    Some(version) if *version == manifest.version => ("Installed", false, false),
                    Some(_) => ("Update", true, false),
                    None => ("Install", true, true),
                };
                let archive = entry.archive.clone();
                ListRow::new(&manifest.name)
                    .icon(Glyph::Plugin)
                    .label_tone(Tone::Strong)
                    .secondary(format!("{}  {vendor}", manifest.version))
                    .trailing(Slot::ui_width(88.0, move |ui| {
                        let button = if primary {
                            Button::primary(label)
                        } else {
                            Button::secondary(label)
                        };
                        if ui
                            .add(button.small(true).min_width(76.0).enabled(enabled))
                            .clicked()
                        {
                            installs.borrow_mut().push(archive.clone());
                        }
                    }))
            }
            Row::Problem(text) => ListRow::new(text).label_tone(Tone::Error).flat(true),
        });
    let to = selected?;
    if Some(to) == from {
        return None;
    }
    let to = dialog::skip_unselectable(rows.len(), from.unwrap_or(0), to, |i| rows[i].selectable());
    (rows[to].selectable() && Some(to) != from).then_some(to)
}

fn installed_list(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    let mut rows: Vec<Row<'_>> = Vec::new();
    for group in [Group::Downloaded, Group::Project, Group::Bundled] {
        let entries: Vec<&PluginEntry> = cx
            .state
            .plugin_host
            .registry
            .entries()
            .iter()
            .filter(|entry| group_of(cx.state, entry) == group && matches(entry, &manager.search))
            .collect();
        if entries.is_empty() {
            continue;
        }
        let enabled_count = entries.iter().filter(|entry| enabled(entry)).count();
        let title = match group {
            Group::Downloaded => {
                format!("Downloaded ({enabled_count} of {} enabled)", entries.len())
            }
            Group::Project => "Project".to_owned(),
            Group::Bundled => "Bundled".to_owned(),
        };
        let toggleable: Vec<String> = entries
            .iter()
            .filter(|entry| entry.can_toggle())
            .map(|entry| entry.id().to_owned())
            .collect();
        let all_enabled = enabled_count == entries.len();
        rows.push(Row::Group {
            title,
            link: (!toggleable.is_empty()).then_some((!all_enabled, toggleable)),
        });
        rows.extend(entries.into_iter().map(Row::Installed));
    }
    if !cx.state.plugin_host.registry.problems.is_empty() {
        rows.push(Row::Group {
            title: "Problems".into(),
            link: None,
        });
        for (path, problem) in &cx.state.plugin_host.registry.problems {
            rows.push(Row::Problem(format!("{}: {problem}", path.display())));
        }
    }
    let installs = std::cell::RefCell::new(Vec::<PathBuf>::new());
    let toggles = std::cell::RefCell::new(Vec::<(String, bool)>::new());
    let current = manager.selected.clone();
    let chosen = plugin_list(
        ui,
        "plugin_manager_list",
        "Installed plugins",
        &rows,
        |row| matches!(row, Row::Installed(entry) if Some(entry.id()) == current.as_deref()),
        &installs,
        &toggles,
    );
    if let Some(Row::Installed(entry)) = chosen.map(|index| &rows[index]) {
        manager.selected = Some(entry.id().to_owned());
        manager.detail = DetailTab::Overview;
    }
    for (id, enable) in toggles.into_inner() {
        let command = if enable {
            "plugin.enable"
        } else {
            "plugin.disable"
        };
        cx.dispatch(command, json!({"id": id}));
    }
}

fn placeholder(ui: &mut egui::Ui, text: &str) {
    EmptyState::new(text).show(ui);
}

/// The plugin title, with its avatar, and the vendor line with the homepage link.
fn title_block(ui: &mut egui::Ui, manifest: &jetthing_plugins::PluginManifest) {
    let colors = Theme::colors(ui.ctx());
    ui.horizontal(|ui| {
        ui.add(Avatar::new(manifest.name.clone()).size(36.0));
        ui.vertical(|ui| {
            ui.label(
                RichText::new(&manifest.name)
                    .font(typography::semibold(ui.ctx(), 20.0))
                    .color(colors.text_strong),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if manifest.vendor.is_empty() {
                        "Unknown vendor"
                    } else {
                        &manifest.vendor
                    })
                    .color(colors.text_secondary),
                );
                if let Some(homepage) = &manifest.homepage
                    && ui.add(Link::new("Plugin homepage ↗")).clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(homepage));
                }
            });
        });
    });
}

/// Overview / Settings / Additional Info tabs of the details pane.
fn detail_tabs(ui: &mut egui::Ui, manager: &mut PluginManager, tabs: &[(DetailTab, &str)]) {
    if !tabs.iter().any(|(tab, _)| *tab == manager.detail) {
        manager.detail = DetailTab::Overview;
    }
    let bar: Vec<KitTab> = tabs.iter().map(|(_, label)| KitTab::new(*label)).collect();
    let current = tabs
        .iter()
        .position(|(tab, _)| *tab == manager.detail)
        .unwrap_or(0);
    if let Some(index) = TabBar::new(&bar, current).show(ui).selected {
        manager.detail = tabs[index].0;
    }
}

fn market_details(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    let entry = manager.catalog.as_ref().and_then(|catalog| {
        catalog
            .entries
            .iter()
            .find(|entry| Some(entry.manifest.id.as_str()) == manager.market_selected.as_deref())
    });
    let Some(entry) = entry.cloned() else {
        plugin_directories_with_registries(cx, ui);
        return;
    };
    let manifest = &entry.manifest;
    title_block(ui, manifest);
    ui.add_space(8.0);
    let installed_version = cx
        .state
        .plugin_host
        .registry
        .get(&manifest.id)
        .map(|installed| installed.manifest.version.clone());
    ui.horizontal(|ui| {
        let (label, enabled) = match &installed_version {
            Some(version) if *version == manifest.version => ("Installed", false),
            Some(_) => ("Update", true),
            None => ("Install", true),
        };
        let button = if enabled && installed_version.is_none() {
            Button::primary(label)
        } else {
            Button::secondary(label)
        };
        if ui.add(button.enabled(enabled)).clicked() {
            manager.install_error = None;
            cx.dispatch(
                "plugin.install",
                json!({"path": entry.archive.display().to_string()}),
            );
        }
        ui.label(RichText::new(&manifest.version).color(Theme::colors(ui.ctx()).text));
        if let Some(installed) = &installed_version
            && *installed != manifest.version
        {
            dialog::hint(ui, format!("(installed {installed})"));
        }
    });
    if let Some(error) = &manager.install_error {
        Banner::new(Severity::Error, dialog::friendly("Plugin file", error)).show(ui);
    }
    ui.add_space(8.0);
    detail_tabs(
        ui,
        manager,
        &[
            (DetailTab::Overview, "Overview"),
            (DetailTab::AdditionalInfo, "Additional Info"),
        ],
    );
    egui::ScrollArea::vertical()
        .id_salt(("plugin_market_details", &manifest.id))
        .auto_shrink([false, false])
        .show(ui, |ui| match manager.detail {
            DetailTab::AdditionalInfo => {
                let mut grid = PropertyGrid::new("plugin_market_info");
                for (key, value) in additional_info(manifest, &entry.archive.display().to_string())
                {
                    grid = grid.row(key, value);
                }
                grid.show(ui);
            }
            _ => {
                if manifest.description.is_empty() {
                    dialog::hint(ui, "No description.");
                } else {
                    dialog::message(ui, manifest.description.clone());
                }
                ui.add(SectionHeader::new("Requested permissions"));
                if manifest.permissions.is_empty() {
                    dialog::hint(ui, "None");
                }
                for permission in &manifest.permissions {
                    dialog::message(ui, format!("• {}", permission.description()));
                }
            }
        });
}

fn plugin_directories_with_registries(cx: &PageContext<'_>, ui: &mut egui::Ui) {
    ui.add(SectionHeader::new("Plugin registries"));
    dialog::hint(
        ui,
        "The Marketplace lists the .jtplugin archives found in these directories \
         (JETTHING_REGISTRY_DIR overrides the first entry):",
    );
    for dir in jetthing_core::plugin_registry_dirs() {
        dialog::message(ui, dir.display().to_string());
    }
    ui.add_space(12.0);
    plugin_directories(cx, ui);
}

/// Key/value rows of the Additional Info tab shared by both tabs' details.
fn additional_info(
    manifest: &jetthing_plugins::PluginManifest,
    source: &str,
) -> Vec<(&'static str, String)> {
    let list = |items: Vec<String>| {
        if items.is_empty() {
            "None".to_owned()
        } else {
            items.join(", ")
        }
    };
    vec![
        ("Plugin ID", manifest.id.clone()),
        ("Kind", manifest.kind.label().to_owned()),
        ("API version", manifest.api_version.to_string()),
        ("Source", source.to_owned()),
        (
            "License",
            manifest
                .license
                .clone()
                .unwrap_or_else(|| "Not specified".into()),
        ),
        ("Dependencies", list(manifest.dependencies.clone())),
        ("Tags", list(manifest.capabilities.clone())),
    ]
}

fn details(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    let Some(entry) = manager
        .selected
        .as_deref()
        .and_then(|id| cx.state.plugin_host.registry.get(id))
    else {
        placeholder(ui, "Select plugin to preview details");
        return;
    };
    let manifest = entry.manifest.clone();
    let contributions = entry.contributions.clone();
    let status = entry.status.clone();
    let source = entry.source.clone();
    let can_toggle = entry.can_toggle();
    let is_enabled = enabled(entry);
    let uninstallable = cx.state.plugin_uninstallable(&manifest.id);
    title_block(ui, &manifest);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let label = if is_enabled { "Disable" } else { "Enable" };
        if ui
            .add(Button::secondary(label).enabled(can_toggle))
            .clicked()
        {
            let command = if is_enabled {
                "plugin.disable"
            } else {
                "plugin.enable"
            };
            cx.dispatch(command, json!({"id": manifest.id}));
        }
        if uninstallable && ui.add(Button::secondary("Uninstall")).clicked() {
            cx.dispatch("plugin.uninstall", json!({"id": manifest.id}));
            manager.selected = None;
        }
        ui.label(RichText::new(&manifest.version).color(Theme::colors(ui.ctx()).text));
    });
    if let PluginStatus::Failed(reason) | PluginStatus::Unsupported(reason) = &status {
        let severity = if matches!(status, PluginStatus::Failed(_)) {
            Severity::Error
        } else {
            Severity::Warning
        };
        Banner::new(severity, reason.clone()).show(ui);
    }
    ui.add_space(8.0);
    let mut tabs = vec![(DetailTab::Overview, "Overview")];
    if !contributions.settings_pages.is_empty() {
        tabs.push((DetailTab::Settings, "Settings"));
    }
    tabs.push((DetailTab::AdditionalInfo, "Additional Info"));
    detail_tabs(ui, manager, &tabs);
    egui::ScrollArea::vertical()
        .id_salt(("plugin_details", &manifest.id))
        .auto_shrink([false, false])
        .show(ui, |ui| match manager.detail {
            DetailTab::Overview => {
                if manifest.description.is_empty() {
                    dialog::hint(ui, "No description.");
                } else {
                    dialog::message(ui, manifest.description.clone());
                }
                let items = [
                    (contributions.commands.len(), "command", "commands"),
                    (
                        contributions.tool_windows.len(),
                        "tool window",
                        "tool windows",
                    ),
                    (contributions.editors.len(), "editor view", "editor views"),
                    (
                        contributions.settings_pages.len(),
                        "settings page",
                        "settings pages",
                    ),
                    (
                        contributions.status_widgets.len(),
                        "status bar widget",
                        "status bar widgets",
                    ),
                    (contributions.keybindings.len(), "keybinding", "keybindings"),
                    (contributions.menus.len(), "menu item", "menu items"),
                ];
                if items.iter().any(|(count, ..)| *count > 0) {
                    ui.add(SectionHeader::new("Features"));
                    for (count, one, many) in items.into_iter().filter(|(count, ..)| *count > 0) {
                        dialog::message(ui, format!("• {}", widgets::pluralize(count, one, many)));
                    }
                }
                if manifest.app.is_some() {
                    dialog::hint(
                        ui,
                        format!("Run standalone: jetthing --app {}", manifest.id),
                    );
                }
            }
            DetailTab::Settings => settings(cx, ui, &manifest.id, &contributions),
            DetailTab::AdditionalInfo => {
                let source = match &source {
                    PluginSource::Bundled => "Bundled with Jetthing".to_owned(),
                    PluginSource::Directory(dir) => dir.display().to_string(),
                };
                let permissions = if manifest.permissions.is_empty() {
                    "None".to_owned()
                } else {
                    manifest
                        .permissions
                        .iter()
                        .map(|p| p.description())
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let mut grid = PropertyGrid::new("plugin_info");
                let mut rows = additional_info(&manifest, &source);
                rows.insert(3, ("Status", status.label().to_owned()));
                rows.insert(6, ("Permissions", permissions));
                for (key, value) in rows {
                    grid = grid.row(key, value);
                }
                grid.show(ui);
            }
        });
}

/// Declarative settings pages of a plugin; edits apply immediately.
fn settings(
    cx: &mut PageContext<'_>,
    ui: &mut egui::Ui,
    plugin: &str,
    contributions: &jetthing_plugins::Contributions,
) {
    for page in &contributions.settings_pages {
        ui.add(SectionHeader::new(page.title.clone()));
        for field in &page.fields {
            let current = cx
                .state
                .plugin_setting(plugin, &field.key)
                .unwrap_or_else(|| field.kind.default_value());
            let mut updated = None;
            match &field.kind {
                SettingKind::Text { .. } => {
                    let mut text = current.as_str().unwrap_or_default().to_owned();
                    if ui
                        .add(TextField::new(&mut text).label(field.label.clone()))
                        .changed()
                    {
                        updated = Some(Value::from(text));
                    }
                }
                SettingKind::Bool { .. } => {
                    let mut flag = current.as_bool().unwrap_or_default();
                    if ui
                        .add(Checkbox::new(&mut flag, field.label.clone()))
                        .changed()
                    {
                        updated = Some(Value::from(flag));
                    }
                }
                SettingKind::Number { min, max, .. } => {
                    // The text being typed is kept until it is a valid number in range.
                    let buffer_id = egui::Id::new(("plugin_setting_number", plugin, &field.key));
                    let mut text: String = ui
                        .data(|d| d.get_temp(buffer_id))
                        .unwrap_or_else(|| current.as_f64().unwrap_or_default().to_string());
                    let parsed = text.trim().parse::<f64>().ok();
                    let in_range = parsed.is_some_and(|n| {
                        min.is_none_or(|min| n >= min) && max.is_none_or(|max| n <= max)
                    });
                    let error = if in_range {
                        None
                    } else {
                        Some(match (min, max) {
                            (Some(min), Some(max)) => {
                                format!("{} must be a number from {min} to {max}.", field.label)
                            }
                            _ => format!("{} must be a number.", field.label),
                        })
                    };
                    let response = ui.add(
                        TextField::new(&mut text)
                            .label(field.label.clone())
                            .error(error.as_deref()),
                    );
                    if response.changed()
                        && in_range
                        && let Some(number) = text.trim().parse::<f64>().ok()
                    {
                        updated = Some(Value::from(number));
                    }
                    ui.data_mut(|d| d.insert_temp(buffer_id, text));
                }
                SettingKind::Choice { options, .. } => {
                    let mut selected = current.as_str().unwrap_or_default().to_owned();
                    let before = selected.clone();
                    ui.add(
                        Dropdown::new(
                            ("plugin_setting", plugin, &field.key),
                            &mut selected,
                            options.clone(),
                        )
                        .label(field.label.clone())
                        .width(240.0),
                    );
                    if selected != before {
                        updated = Some(Value::from(selected));
                    }
                }
            }
            if let Some(description) = &field.description {
                dialog::hint(ui, description.clone());
            }
            if let Some(value) = updated {
                cx.dispatch(
                    "plugin.setSetting",
                    json!({"plugin": plugin, "key": field.key, "value": value}),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jetthing_plugins::{Plugin, PluginManifest, PluginRegistry};
    use std::rc::Rc;

    struct P(PluginManifest);

    impl Plugin for P {
        fn manifest(&self) -> &PluginManifest {
            &self.0
        }
    }

    #[test]
    fn search_matches_name_vendor_and_tags() {
        let mut registry = PluginRegistry::default();
        registry
            .add_bundled(Rc::new(|| {
                Box::new(P(PluginManifest::parse(
                    "id='x.http'\nname='Nexus HTTP'\nversion='1'\napi_version=1\nkind='rust'\nvendor='Jetthing'\ncapabilities=['rest']\n",
                )
                .unwrap()))
            }))
            .unwrap();
        let entry = &registry.entries()[0];
        for query in ["", "nexus", "JETTHING", "rest", "x.http"] {
            assert!(matches(entry, query), "{query}");
        }
        assert!(!matches(entry, "mongo"));
    }
}
