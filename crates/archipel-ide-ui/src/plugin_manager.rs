//! Settings › Plugins page (observable layout of the reference scene
//! `24-plugins`), registered as the `preferences.pluginManager` settings page:
//! Marketplace / Installed tabs with a gear menu, a searchable grouped plugin list
//! (downloaded, project and bundled plugins from the plugin registry, plus manifests
//! that failed to load), and a details pane with Enable/Disable, Uninstall and
//! Overview / Settings / Additional Info tabs. The page reads the IDE state and
//! queues commands (`plugin.enable`, `plugin.disable`, `plugin.install`,
//! `plugin.uninstall`, `plugin.refresh`, `plugin.setSetting`) that the settings
//! dialog runs after drawing.

use crate::icons;
use crate::theme;
use crate::widgets;
use archipel_ide_core::IdeState;
use archipel_ide_plugins::catalog::{Catalog, CatalogEntry};
use archipel_ide_plugins::{PluginEntry, PluginSource, PluginStatus, SettingKind};
use eframe::egui::{self, Color32, Rect, RichText, Sense, vec2};
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
const ROW_HEIGHT: f32 = 64.0;
const GROUP_HEIGHT: f32 = 26.0;

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
        // `ARCHIPEL_PLUGINS_DIR` and `plugins/` next to the executable.
        PluginSource::Directory(_) => Group::Bundled,
    }
}

fn matches(entry: &PluginEntry, query: &str) -> bool {
    matches_manifest(&entry.manifest, query)
}

fn matches_manifest(manifest: &archipel_ide_plugins::PluginManifest, query: &str) -> bool {
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
    {
        // The guard ends with this block: the rows below dispatch commands that write the
        // registry.
        let plugins = cx.state.plugin_host_service();
        let registry = plugins.registry();
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
    }
    if manager.tab == Tab::Marketplace && manager.catalog.is_none() {
        manager.catalog = Some(Catalog::scan(&archipel_ide_core::plugin_registry_dirs()));
    }
    let full = ui.available_rect_before_wrap();
    ui.allocate_rect(full, Sense::hover());
    let header = Rect::from_min_size(full.min, vec2(full.width(), 40.0));
    let body = Rect::from_min_max(egui::pos2(full.left(), header.bottom()), full.max);
    header_row(
        cx,
        &mut ui.new_child(egui::UiBuilder::new().max_rect(header)),
        manager,
    );
    let painter = ui.painter();
    painter.hline(
        full.x_range(),
        header.bottom(),
        egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
    );
    let list_rect = Rect::from_min_max(
        body.min,
        egui::pos2(body.left() + LIST_WIDTH, body.bottom()),
    );
    let details_rect = Rect::from_min_max(egui::pos2(list_rect.right(), body.top()), body.max);
    painter.vline(
        list_rect.right(),
        body.y_range(),
        egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
    );
    let mut list_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(list_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    list(cx, &mut list_ui, manager);
    let mut details_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(details_rect.shrink2(vec2(20.0, 14.0)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    match manager.tab {
        Tab::Installed => details(cx, &mut details_ui, manager),
        Tab::Marketplace => market_details(cx, &mut details_ui, manager),
    }
}

/// Where plugins can be installed from, shown when nothing is selected.
fn plugin_directories(cx: &PageContext<'_>, ui: &mut egui::Ui) {
    ui.label(
        RichText::new("Plugin directories")
            .font(theme::semibold(theme::UI_FONT_SIZE + 2.0))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Archipel IDE loads plugins (a folder with an archipel-plugin.toml) from these directories; \
             .archipelplugin archives install into the User directory:",
        )
        .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(4.0);
    let project = cx.state.project().map(|p| p.root());
    for (source, dir) in archipel_ide_core::plugin_dirs(project) {
        ui.horizontal(|ui| {
            ui.add_sized(
                [70.0, 18.0],
                egui::Label::new(RichText::new(source).color(theme::TEXT_INFO)),
            );
            ui.label(RichText::new(dir.display().to_string()).color(theme::TEXT));
        });
    }
}

fn header_row(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    let rect = ui.max_rect();
    let installed = cx.state.plugin_host_service().registry().entries().len();
    let font = egui::FontId::proportional(theme::UI_FONT_SIZE);
    let mut x = rect.left();
    for (tab, label) in [
        (Tab::Marketplace, "Marketplace"),
        (Tab::Installed, "Installed"),
    ] {
        let galley = ui
            .painter()
            .layout_no_wrap(label.into(), font.clone(), *theme::TEXT);
        let badge = (tab == Tab::Installed).then(|| installed.to_string());
        let width = galley.size().x + 24.0 + if badge.is_some() { 22.0 } else { 0.0 };
        let tab_rect = Rect::from_min_size(egui::pos2(x, rect.top()), vec2(width, rect.height()));
        let response = ui.interact(
            tab_rect,
            egui::Id::new(("plugin_tab_header", label)),
            Sense::click(),
        );
        let active = manager.tab == tab;
        if response.hovered() && !active {
            ui.painter()
                .rect_filled(tab_rect.shrink2(vec2(0.0, 6.0)), 4.0, theme::HOVER);
        }
        let text_color = if active {
            theme::TEXT_BRIGHT
        } else {
            theme::TEXT
        };
        let text_pos = egui::pos2(
            tab_rect.left() + 12.0,
            tab_rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(text_pos, galley.clone(), *text_color);
        if let Some(badge) = badge {
            let center = egui::pos2(text_pos.x + galley.size().x + 13.0, tab_rect.center().y);
            ui.painter().circle_filled(center, 8.0, theme::TEXT_INFO);
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                badge,
                egui::FontId::proportional(10.0),
                *theme::POPUP,
            );
        }
        if active {
            ui.painter().rect_filled(
                Rect::from_min_max(
                    egui::pos2(tab_rect.left(), tab_rect.bottom() - 3.0),
                    tab_rect.right_bottom(),
                ),
                0.0,
                theme::ACCENT,
            );
        }
        if response.clicked() {
            manager.tab = tab;
            if tab == Tab::Marketplace {
                manager.catalog = None; // rescan the registries
            }
        }
        x += width + 4.0;
    }
    let gear_rect = Rect::from_center_size(egui::pos2(x + 18.0, rect.center().y), vec2(24.0, 24.0));
    let gear = ui.interact(
        gear_rect,
        egui::Id::new("plugin_manager_gear"),
        Sense::click(),
    );
    if gear.hovered() {
        ui.painter().rect_filled(gear_rect, 4.0, theme::HOVER);
    }
    paint_gear(ui.painter(), gear_rect.center(), *icons::ICON);
    let gear = gear.on_hover_text("Manage plugins");
    egui::Popup::menu(&gear).show(|ui| {
        ui.set_min_width(220.0);
        if ui.button("Install Plugin from Disk…").clicked() {
            manager.tab = Tab::Installed;
            manager.install_path = Some(String::new());
            manager.install_error = None;
            ui.close();
        }
        if ui.button("Refresh Plugin List").clicked() {
            manager.catalog = None;
            cx.dispatch("plugin.refresh", Value::Null);
            ui.close();
        }
    });
}

/// Original gear glyph: a ring with eight teeth.
fn paint_gear(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    let stroke = egui::Stroke::new(1.3_f32, color);
    painter.circle_stroke(center, 4.5, stroke);
    painter.circle_stroke(center, 1.8, stroke);
    for tooth in 0..8 {
        let angle = tooth as f32 * std::f32::consts::FRAC_PI_4;
        let direction = vec2(angle.cos(), angle.sin());
        painter.line_segment([center + direction * 5.0, center + direction * 7.0], stroke);
    }
}

fn placeholder(ui: &mut egui::Ui, text: &str) {
    ui.centered_and_justified(|ui| {
        ui.label(RichText::new(text).color(theme::TEXT_SECONDARY));
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
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Install Plugin from Disk")
                        .font(theme::semibold(theme::UI_FONT_SIZE)),
                );
                let response = ui.add(
                    egui::TextEdit::singleline(path)
                        .hint_text("/path/to/plugin.archipelplugin")
                        .desired_width(LIST_WIDTH - 24.0),
                );
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    install = true;
                }
                ui.horizontal(|ui| {
                    install |= ui.button("Install").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
                if let Some(error) = &manager.install_error {
                    ui.label(RichText::new(error).small().color(theme::ERROR));
                }
            });
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
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            let (title, hint) = if catalog.entries.is_empty() {
                (
                    "No plugins in the local registry",
                    "Put .archipelplugin archives into one of the registry\ndirectories listed on the right.",
                )
            } else {
                ("Nothing found", "No registry plugin matches the search.")
            };
            ui.label(RichText::new(title).color(theme::TEXT_BRIGHT));
            ui.add_space(4.0);
            ui.label(RichText::new(hint).color(theme::TEXT_SECONDARY));
            ui.add_space(8.0);
            if ui.button("Install Plugin from Disk…").clicked() {
                manager.tab = Tab::Installed;
                manager.install_path = Some(String::new());
            }
        });
        return;
    }
    let mut installs: Vec<PathBuf> = Vec::new();
    egui::ScrollArea::vertical()
        .id_salt("plugin_market_list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            group_header(ui, "Local registry", true, false);
            for entry in visible {
                let installed = cx.state.plugin_host_service().version(&entry.manifest.id);
                if market_row(ui, entry, installed.as_deref(), manager) {
                    installs.push(entry.archive.clone());
                }
            }
        });
    for archive in installs {
        manager.install_error = None;
        cx.dispatch(
            "plugin.install",
            json!({"path": archive.display().to_string()}),
        );
    }
}

/// One Marketplace card: icon, name, version and vendor, Install/Installed.
/// Returns true when Install was clicked.
fn market_row(
    ui: &mut egui::Ui,
    entry: &CatalogEntry,
    installed: Option<&str>,
    manager: &mut PluginManager,
) -> bool {
    let manifest = &entry.manifest;
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let selected = manager.market_selected.as_deref() == Some(manifest.id.as_str());
    if selected {
        ui.painter().rect_filled(rect, 0.0, theme::SELECTION);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::HOVER);
    }
    paint_plugin_icon(
        ui.painter(),
        Rect::from_min_size(rect.left_center() + vec2(10.0, -18.0), vec2(36.0, 36.0)),
        &manifest.name,
    );
    let text_left = rect.left() + 58.0;
    let name = ui.painter().layout(
        manifest.name.clone(),
        theme::semibold(theme::UI_FONT_SIZE),
        *theme::TEXT_BRIGHT,
        rect.width() - 150.0,
    );
    ui.painter().galley(
        egui::pos2(text_left, rect.top() + 14.0),
        name,
        *theme::TEXT_BRIGHT,
    );
    let vendor = if manifest.vendor.is_empty() {
        "Unknown vendor"
    } else {
        &manifest.vendor
    };
    ui.painter().text(
        egui::pos2(text_left, rect.top() + 36.0),
        egui::Align2::LEFT_TOP,
        format!("{}  {vendor}", manifest.version),
        egui::FontId::proportional(theme::UI_FONT_SIZE - 0.5),
        *theme::TEXT_INFO,
    );
    let button_rect = Rect::from_center_size(
        egui::pos2(rect.right() - 46.0, rect.center().y),
        vec2(76.0, 26.0),
    );
    let mut install = false;
    let mut button_ui = ui.new_child(egui::UiBuilder::new().max_rect(button_rect));
    match installed {
        Some(version) if version == manifest.version => {
            button_ui.add_enabled(
                false,
                egui::Button::new("Installed").min_size(button_rect.size()),
            );
        }
        Some(_) => {
            install = button_ui
                .add(egui::Button::new("Update").min_size(button_rect.size()))
                .clicked()
        }
        None => {
            install = button_ui
                .add(egui::Button::new("Install").min_size(button_rect.size()))
                .clicked()
        }
    }
    if response.clicked() {
        manager.market_selected = Some(manifest.id.clone());
        manager.detail = DetailTab::Overview;
    }
    install
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
    ui.label(
        RichText::new(&manifest.name)
            .font(theme::semibold(20.0))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(if manifest.vendor.is_empty() {
                "Unknown vendor"
            } else {
                &manifest.vendor
            })
            .color(theme::TEXT_SECONDARY),
        );
        if let Some(homepage) = &manifest.homepage {
            ui.hyperlink_to("Plugin homepage ↗", homepage);
        }
    });
    ui.add_space(8.0);
    let installed_version = cx
        .state
        .plugin_host_service()
        .registry()
        .get(&manifest.id)
        .map(|installed| installed.manifest.version.clone());
    ui.horizontal(|ui| {
        let (label, enabled) = match &installed_version {
            Some(version) if *version == manifest.version => ("Installed", false),
            Some(_) => ("Update", true),
            None => ("Install", true),
        };
        if ui
            .add_enabled(enabled, egui::Button::new(label).min_size(vec2(84.0, 28.0)))
            .clicked()
        {
            manager.install_error = None;
            cx.dispatch(
                "plugin.install",
                json!({"path": entry.archive.display().to_string()}),
            );
        }
        ui.add_space(8.0);
        ui.label(RichText::new(&manifest.version).color(theme::TEXT));
        if let Some(installed) = &installed_version
            && *installed != manifest.version
        {
            ui.label(
                RichText::new(format!("(installed {installed})")).color(theme::TEXT_SECONDARY),
            );
        }
    });
    if let Some(error) = &manager.install_error {
        ui.add_space(6.0);
        ui.label(RichText::new(error).color(theme::ERROR));
    }
    ui.add_space(10.0);
    detail_tabs(
        ui,
        manager,
        &[
            (DetailTab::Overview, "Overview"),
            (DetailTab::AdditionalInfo, "Additional Info"),
        ],
    );
    ui.separator();
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .id_salt(("plugin_market_details", &manifest.id))
        .auto_shrink([false, false])
        .show(ui, |ui| match manager.detail {
            DetailTab::AdditionalInfo => {
                let rows = additional_info(manifest, &entry.archive.display().to_string());
                for (key, value) in rows {
                    info_row(ui, key, &value);
                }
            }
            _ => {
                ui.label(if manifest.description.is_empty() {
                    RichText::new("No description.").color(theme::TEXT_SECONDARY)
                } else {
                    RichText::new(&manifest.description).color(theme::TEXT)
                });
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Requested permissions")
                        .font(theme::semibold(theme::UI_FONT_SIZE)),
                );
                if manifest.permissions.is_empty() {
                    ui.label(RichText::new("None").color(theme::TEXT_SECONDARY));
                }
                for permission in &manifest.permissions {
                    ui.label(format!("• {}", permission.description()));
                }
            }
        });
}

fn plugin_directories_with_registries(cx: &PageContext<'_>, ui: &mut egui::Ui) {
    ui.label(
        RichText::new("Plugin registries")
            .font(theme::semibold(theme::UI_FONT_SIZE + 2.0))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "The Marketplace lists the .archipelplugin archives found in these directories \
             (ARCHIPEL_REGISTRY_DIR overrides the first entry):",
        )
        .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(4.0);
    for dir in archipel_ide_core::plugin_registry_dirs() {
        ui.label(RichText::new(dir.display().to_string()).color(theme::TEXT));
    }
    ui.add_space(12.0);
    plugin_directories(cx, ui);
}

fn info_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [110.0, 18.0],
            egui::Label::new(RichText::new(key).color(theme::TEXT_SECONDARY)),
        );
        ui.label(value);
    });
    ui.add_space(4.0);
}

/// Key/value rows of the Additional Info tab shared by both tabs' details.
fn additional_info(
    manifest: &archipel_ide_plugins::PluginManifest,
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

fn detail_tabs(ui: &mut egui::Ui, manager: &mut PluginManager, tabs: &[(DetailTab, &str)]) {
    if !tabs.iter().any(|(tab, _)| *tab == manager.detail) {
        manager.detail = DetailTab::Overview;
    }
    ui.horizontal(|ui| {
        for (tab, label) in tabs {
            let active = manager.detail == *tab;
            let text = RichText::new(*label).color(if active {
                theme::TEXT_BRIGHT
            } else {
                theme::TEXT
            });
            let response = ui.add(
                egui::Button::new(text)
                    .frame(false)
                    .min_size(vec2(0.0, 28.0)),
            );
            if active {
                let r = response.rect;
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        egui::pos2(r.left(), r.bottom() + 1.0),
                        egui::pos2(r.right(), r.bottom() + 4.0),
                    ),
                    0.0,
                    theme::ACCENT,
                );
            }
            if response.clicked() {
                manager.detail = *tab;
            }
            ui.add_space(8.0);
        }
    });
}

fn installed_list(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    let mut toggles: Vec<(String, bool)> = Vec::new();
    egui::ScrollArea::vertical()
        .id_salt("plugin_manager_list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for group in [Group::Downloaded, Group::Project, Group::Bundled] {
                let plugins = cx.state.plugin_host_service();
                let registry = plugins.registry();
                let entries: Vec<&PluginEntry> = registry
                    .entries()
                    .iter()
                    .filter(|entry| {
                        group_of(cx.state, entry) == group && matches(entry, &manager.search)
                    })
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
                if let Some(enable) = group_header(ui, &title, all_enabled, !toggleable.is_empty())
                {
                    toggles.extend(toggleable.into_iter().map(|id| (id, enable)));
                }
                for entry in entries {
                    if let Some(toggle) = plugin_row(ui, entry, manager) {
                        toggles.push((entry.id().to_owned(), toggle));
                    }
                }
            }
            if !cx
                .state
                .plugin_host_service()
                .registry()
                .problems
                .is_empty()
            {
                group_header(ui, "Problems", true, false);
                for (path, problem) in &cx.state.plugin_host_service().registry().problems {
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("{}: {problem}", path.display()))
                                    .small()
                                    .color(theme::ERROR),
                            );
                        });
                }
            }
        });
    for (id, enable) in toggles {
        let command = if enable {
            "plugin.enable"
        } else {
            "plugin.disable"
        };
        cx.dispatch(command, json!({"id": id}));
    }
}

/// Group header with an optional "Disable all"/"Enable all" link; returns
/// the requested state when the link is clicked.
fn group_header(ui: &mut egui::Ui, title: &str, all_enabled: bool, link: bool) -> Option<bool> {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), GROUP_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::NOTIFICATION);
    ui.painter().text(
        rect.left_center() + vec2(10.0, 0.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT,
    );
    if !link {
        return None;
    }
    let label = if all_enabled {
        "Disable all"
    } else {
        "Enable all"
    };
    let galley = ui.painter().layout_no_wrap(
        label.into(),
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::MODIFIED,
    );
    let link_rect = Rect::from_min_size(
        egui::pos2(
            rect.right() - galley.size().x - 10.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley.size(),
    );
    let response = ui.interact(
        link_rect,
        egui::Id::new(("plugin_group_link", title)),
        Sense::click(),
    );
    ui.painter().galley(link_rect.min, galley, *theme::MODIFIED);
    response.clicked().then_some(!all_enabled)
}

/// One plugin row: icon, bold name, version and vendor, checkbox. Returns a
/// requested enable state when the checkbox was toggled.
fn plugin_row(ui: &mut egui::Ui, entry: &PluginEntry, manager: &mut PluginManager) -> Option<bool> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let selected = manager.selected.as_deref() == Some(entry.id());
    if selected {
        ui.painter().rect_filled(rect, 0.0, theme::SELECTION);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::HOVER);
    }
    let manifest = &entry.manifest;
    paint_plugin_icon(
        ui.painter(),
        Rect::from_min_size(rect.left_center() + vec2(10.0, -18.0), vec2(36.0, 36.0)),
        &manifest.name,
    );
    let muted = !enabled(entry);
    let name_color = match entry.status {
        PluginStatus::Failed(_) => theme::ERROR,
        _ if muted => theme::TEXT_SECONDARY,
        _ => theme::TEXT_BRIGHT,
    };
    let text_left = rect.left() + 58.0;
    let name = ui.painter().layout(
        manifest.name.clone(),
        theme::semibold(theme::UI_FONT_SIZE),
        *name_color,
        rect.width() - 100.0,
    );
    ui.painter()
        .galley(egui::pos2(text_left, rect.top() + 14.0), name, *name_color);
    let detail = match &entry.status {
        PluginStatus::Failed(_) => format!("{}  failed to load", manifest.version),
        PluginStatus::Unsupported(_) => format!("{}  unsupported", manifest.version),
        _ if entry.source == PluginSource::Bundled => format!("{}  bundled", manifest.version),
        _ => format!("{}  {}", manifest.version, manifest.vendor),
    };
    ui.painter().text(
        egui::pos2(text_left, rect.top() + 36.0),
        egui::Align2::LEFT_TOP,
        detail,
        egui::FontId::proportional(theme::UI_FONT_SIZE - 0.5),
        *theme::TEXT_INFO,
    );
    let mut checked = enabled(entry);
    let check_rect = Rect::from_center_size(
        egui::pos2(rect.right() - 24.0, rect.top() + 22.0),
        vec2(18.0, 18.0),
    );
    let mut toggle = None;
    let mut check_ui = ui.new_child(egui::UiBuilder::new().max_rect(check_rect));
    if check_ui
        .add_enabled(
            entry.can_toggle(),
            egui::Checkbox::without_text(&mut checked),
        )
        .changed()
    {
        toggle = Some(checked);
    }
    if response.clicked() {
        manager.selected = Some(entry.id().to_owned());
        manager.detail = DetailTab::Overview;
    }
    toggle
}

/// Plugin icon: rounded square with initials (no third-party artwork).
fn paint_plugin_icon(painter: &egui::Painter, rect: Rect, name: &str) {
    painter.rect_filled(rect, 8.0, theme::avatar_color(name));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        theme::project_initials(name),
        theme::semibold(14.0),
        Color32::WHITE,
    );
}

fn details(cx: &mut PageContext<'_>, ui: &mut egui::Ui, manager: &mut PluginManager) {
    let Some(entry) = manager
        .selected
        .as_deref()
        .and_then(|id| cx.state.plugin_host_service().entry(id))
    else {
        placeholder(ui, "Select plugin to preview details");
        return;
    };
    let manifest = entry.manifest.clone();
    let contributions = entry.contributions.clone();
    let status = entry.status.clone();
    let source = entry.source.clone();
    let can_toggle = entry.can_toggle();
    let is_enabled = enabled(&entry);
    let uninstallable = cx.state.plugin_uninstallable(&manifest.id);
    ui.label(
        RichText::new(&manifest.name)
            .font(theme::semibold(20.0))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(if manifest.vendor.is_empty() {
                "Unknown vendor"
            } else {
                &manifest.vendor
            })
            .color(theme::TEXT_SECONDARY),
        );
        if let Some(homepage) = &manifest.homepage {
            ui.hyperlink_to("Plugin homepage ↗", homepage);
        }
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let label = if is_enabled { "Disable" } else { "Enable" };
        if ui
            .add_enabled(
                can_toggle,
                egui::Button::new(label).min_size(vec2(84.0, 28.0)),
            )
            .clicked()
        {
            let command = if is_enabled {
                "plugin.disable"
            } else {
                "plugin.enable"
            };
            cx.dispatch(command, json!({"id": manifest.id}));
        }
        if uninstallable
            && ui
                .add(egui::Button::new("Uninstall").min_size(vec2(84.0, 28.0)))
                .clicked()
        {
            cx.dispatch("plugin.uninstall", json!({"id": manifest.id}));
            manager.selected = None;
        }
        ui.add_space(8.0);
        ui.label(RichText::new(&manifest.version).color(theme::TEXT));
    });
    if let PluginStatus::Failed(reason) | PluginStatus::Unsupported(reason) = &status {
        ui.add_space(8.0);
        egui::Frame::new()
            .fill(if matches!(status, PluginStatus::Failed(_)) {
                theme::ERROR.gamma_multiply(0.2)
            } else {
                *theme::NOTIFICATION
            })
            .corner_radius(4)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(reason).color(theme::TEXT));
            });
    }
    ui.add_space(10.0);
    let mut tabs = vec![(DetailTab::Overview, "Overview")];
    if !contributions.settings_pages.is_empty() {
        tabs.push((DetailTab::Settings, "Settings"));
    }
    tabs.push((DetailTab::AdditionalInfo, "Additional Info"));
    if !tabs.iter().any(|(tab, _)| *tab == manager.detail) {
        manager.detail = DetailTab::Overview;
    }
    ui.horizontal(|ui| {
        for (tab, label) in &tabs {
            let active = manager.detail == *tab;
            let text = RichText::new(*label).color(if active {
                theme::TEXT_BRIGHT
            } else {
                theme::TEXT
            });
            let response = ui.add(
                egui::Button::new(text)
                    .frame(false)
                    .min_size(vec2(0.0, 28.0)),
            );
            if active {
                let r = response.rect;
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        egui::pos2(r.left(), r.bottom() + 1.0),
                        egui::pos2(r.right(), r.bottom() + 4.0),
                    ),
                    0.0,
                    theme::ACCENT,
                );
            }
            if response.clicked() {
                manager.detail = *tab;
            }
            ui.add_space(8.0);
        }
    });
    ui.separator();
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .id_salt(("plugin_details", &manifest.id))
        .auto_shrink([false, false])
        .show(ui, |ui| match manager.detail {
            DetailTab::Overview => {
                ui.label(if manifest.description.is_empty() {
                    RichText::new("No description.").color(theme::TEXT_SECONDARY)
                } else {
                    RichText::new(&manifest.description).color(theme::TEXT)
                });
                ui.add_space(10.0);
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
                    ui.label(RichText::new("Features").font(theme::semibold(theme::UI_FONT_SIZE)));
                    for (count, one, many) in items.into_iter().filter(|(count, ..)| *count > 0) {
                        ui.label(format!("• {}", widgets::pluralize(count, one, many)));
                    }
                }
                if manifest.app.is_some() {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "Run standalone: archipel-ide --app {}",
                            manifest.id
                        ))
                        .monospace()
                        .color(theme::TEXT_SECONDARY),
                    );
                }
            }
            DetailTab::Settings => settings(cx, ui, &manifest.id, &contributions),
            DetailTab::AdditionalInfo => {
                let source = match &source {
                    PluginSource::Bundled => "Bundled with Archipel IDE".to_owned(),
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
                for (key, value) in [
                    ("Plugin ID", manifest.id.clone()),
                    ("Kind", manifest.kind.label().to_owned()),
                    ("API version", manifest.api_version.to_string()),
                    ("Status", status.label().to_owned()),
                    ("Source", source),
                    (
                        "License",
                        manifest
                            .license
                            .clone()
                            .unwrap_or_else(|| "Not specified".into()),
                    ),
                    ("Permissions", permissions),
                    (
                        "Dependencies",
                        if manifest.dependencies.is_empty() {
                            "None".into()
                        } else {
                            manifest.dependencies.join(", ")
                        },
                    ),
                    (
                        "Tags",
                        if manifest.capabilities.is_empty() {
                            "None".into()
                        } else {
                            manifest.capabilities.join(", ")
                        },
                    ),
                ] {
                    ui.horizontal_top(|ui| {
                        ui.add_sized(
                            [110.0, 18.0],
                            egui::Label::new(RichText::new(key).color(theme::TEXT_SECONDARY)),
                        );
                        ui.label(value);
                    });
                    ui.add_space(4.0);
                }
            }
        });
}

/// Declarative settings pages of a plugin; edits apply immediately.
fn settings(
    cx: &mut PageContext<'_>,
    ui: &mut egui::Ui,
    plugin: &str,
    contributions: &archipel_ide_plugins::Contributions,
) {
    for page in &contributions.settings_pages {
        ui.label(RichText::new(&page.title).font(theme::semibold(theme::UI_FONT_SIZE)));
        ui.add_space(4.0);
        for field in &page.fields {
            let current = cx
                .state
                .plugin_setting(plugin, &field.key)
                .unwrap_or_else(|| field.kind.default_value());
            let mut updated = None;
            ui.horizontal(|ui| {
                ui.add_sized([170.0, 20.0], egui::Label::new(&field.label));
                match &field.kind {
                    SettingKind::Text { .. } => {
                        let mut text = current.as_str().unwrap_or_default().to_owned();
                        if ui.text_edit_singleline(&mut text).changed() {
                            updated = Some(Value::from(text));
                        }
                    }
                    SettingKind::Bool { .. } => {
                        let mut flag = current.as_bool().unwrap_or_default();
                        if ui.checkbox(&mut flag, "").changed() {
                            updated = Some(Value::from(flag));
                        }
                    }
                    SettingKind::Number { min, max, .. } => {
                        let mut number = current.as_f64().unwrap_or_default();
                        let mut drag = egui::DragValue::new(&mut number);
                        if let (Some(min), Some(max)) = (min, max) {
                            drag = drag.range(*min..=*max);
                        }
                        if ui.add(drag).changed() {
                            updated = Some(Value::from(number));
                        }
                    }
                    SettingKind::Choice { options, .. } => {
                        let selected = current.as_str().unwrap_or_default().to_owned();
                        egui::ComboBox::from_id_salt(("plugin_setting", plugin, &field.key))
                            .selected_text(&selected)
                            .show_ui(ui, |ui| {
                                for option in options {
                                    if ui.selectable_label(*option == selected, option).clicked() {
                                        updated = Some(Value::from(option.clone()));
                                    }
                                }
                            });
                    }
                }
            });
            if let Some(description) = &field.description {
                ui.label(RichText::new(description).small().color(theme::TEXT_INFO));
            }
            ui.add_space(4.0);
            if let Some(value) = updated {
                cx.dispatch(
                    "plugin.setSetting",
                    json!({"plugin": plugin, "key": field.key, "value": value}),
                );
            }
        }
        ui.add_space(8.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archipel_ide_plugins::{Plugin, PluginManifest, PluginRegistry, PluginRuntime};
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
            .add_bundled(&mut PluginRuntime::default(), Rc::new(|| {
                Box::new(P(PluginManifest::parse(
                    "id='x.http'\nname='Nexus HTTP'\nversion='1'\napi_version=1\nkind='rust'\nvendor='Archipel IDE'\ncapabilities=['rest']\n",
                )
                .unwrap()))
            }))
            .unwrap();
        let entry = &registry.entries()[0];
        for query in ["", "nexus", "ARCHIPEL", "rest", "x.http"] {
            assert!(matches(entry, query), "{query}");
        }
        assert!(!matches(entry, "mongo"));
    }
}
