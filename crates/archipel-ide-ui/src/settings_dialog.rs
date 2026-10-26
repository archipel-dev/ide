//! Settings dialog (⌘, / Ctrl+Alt+S).
//!
//! Layout follows the reference dialog (`docs/parity/evidence/reference/*/08-settings.png`):
//! a search field above the page tree on the left, a breadcrumb header and the page
//! content on the right, and OK / Cancel / Apply at the bottom. Pages and settings come
//! from the `archipel-ide-settings` registry; pending edits live in a [`SettingsDraft`] and
//! are applied through the `settings.apply` command.

use crate::icons::{self, Icon};
use crate::widgets::search_field;
use crate::{IdeApp, theme};
use archipel_ide_settings::{
    PageDef, Scope, SearchResult, SettingDef, SettingKind, SettingValue, SettingsDraft,
    SettingsRegistry, SettingsStore,
};
use eframe::egui::{self, Color32, Rect, RichText, Sense, pos2, vec2};
use serde_json::json;
use std::collections::BTreeSet;

const TREE_WIDTH: f32 = 236.0;
const TREE_ROW: f32 = 25.0;
const FOOTER_HEIGHT: f32 = 54.0;

pub struct SettingsDialog {
    page: String,
    search: String,
    draft: SettingsDraft,
    expanded: BTreeSet<String>,
    back: Vec<String>,
    forward: Vec<String>,
    error: Option<String>,
    focus_search: bool,
    pub(crate) keymap_page: crate::settings_keymap::KeymapPage,
    /// Settings › Plugins (`preferences.pluginManager`).
    pub(crate) plugins_page: crate::plugin_manager::PluginManager,
    /// Settings › Editor › Live Templates (`editing.templates`).
    live_templates_page: crate::settings_live_templates::LiveTemplatesPage,
}

enum Outcome {
    Keep,
    Apply,
    Ok,
    Cancel,
}

impl SettingsDialog {
    pub fn new(registry: &SettingsRegistry, page: &str) -> Self {
        let mut dialog = Self {
            page: String::new(),
            search: String::new(),
            draft: SettingsDraft::default(),
            expanded: BTreeSet::new(),
            back: Vec::new(),
            forward: Vec::new(),
            error: None,
            focus_search: true,
            keymap_page: Default::default(),
            plugins_page: Default::default(),
            live_templates_page: Default::default(),
        };
        dialog.select(registry, page);
        dialog.back.clear();
        dialog
    }

    /// Shows `page`, expanding its ancestors and recording navigation history.
    fn select(&mut self, registry: &SettingsRegistry, page: &str) {
        if registry.page(page).is_none() || self.page == page {
            return;
        }
        if !self.page.is_empty() {
            self.back.push(std::mem::take(&mut self.page));
            self.forward.clear();
        }
        let mut current = registry.page(page).and_then(|p| p.parent.clone());
        while let Some(parent) = current {
            current = registry.page(&parent).and_then(|p| p.parent.clone());
            self.expanded.insert(parent);
        }
        self.page = page.to_owned();
    }

    fn navigate(&mut self, back: bool) {
        let (from, to) = if back {
            (&mut self.back, &mut self.forward)
        } else {
            (&mut self.forward, &mut self.back)
        };
        if let Some(page) = from.pop() {
            to.push(std::mem::replace(&mut self.page, page));
        }
    }
}

/// Opens the dialog when a `settings.open` command asked for it.
pub fn take_request(app: &mut IdeApp) {
    if let Some(page) = app.state.settings_request.take() {
        let search = app.keymap_search.take();
        match app.settings_dialog.as_mut() {
            Some(dialog) => dialog.select(&app.state.settings.registry, &page),
            None => {
                app.settings_dialog = Some(SettingsDialog::new(&app.state.settings.registry, &page))
            }
        }
        if let (Some(search), Some(dialog)) = (search, app.settings_dialog.as_mut()) {
            dialog.keymap_page.set_query(search);
        }
    }
}

fn backdrop_id() -> egui::Id {
    egui::Id::new("settings_backdrop")
}

/// Keeps the dimming backdrop above notification balloons (which share its
/// layer order) while the dialog is open; called after the balloons are shown.
pub fn backdrop_to_top(ctx: &egui::Context) {
    ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, backdrop_id()));
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    take_request(app);
    let Some(mut dialog) = app.settings_dialog.take() else {
        return;
    };
    let mut requests = Vec::new();
    let outcome = draw(
        &mut dialog,
        PageContext {
            registry: &app.state.settings.registry,
            store: &app.state.settings.store,
            keymap: &app.state.keymap,
            state: &app.state,
            requests: &mut requests,
        },
        ctx,
    );
    // Pages that edit through commands (Keymap) queue them while drawing.
    for (id, params) in requests {
        let result = app.commands.execute(&id, &mut app.state, params);
        if id == "plugin.install" {
            // The Plugins page shows install errors next to the path field.
            dialog
                .plugins_page
                .install_finished(result.map_err(|error| error.to_string()));
        } else if let Err(error) = result {
            dialog.error = Some(error.to_string());
        }
    }
    let apply = matches!(outcome, Outcome::Apply | Outcome::Ok) && dialog.draft.is_modified();
    if apply {
        let changes: serde_json::Map<_, _> = dialog
            .draft
            .changes()
            .iter()
            .map(|(id, value)| (id.clone(), value.to_json()))
            .collect();
        match app.commands.execute(
            "settings.apply",
            &mut app.state,
            json!({"changes": changes}),
        ) {
            Ok(_) => {
                dialog.draft = SettingsDraft::default();
                dialog.error = None;
            }
            Err(error) => dialog.error = Some(error.to_string()),
        }
    }
    let close = match outcome {
        Outcome::Cancel => true,
        Outcome::Ok => dialog.error.is_none(),
        Outcome::Keep | Outcome::Apply => false,
    };
    if !close {
        app.settings_dialog = Some(dialog);
    }
}

/// What a page may read and the commands it may queue.
struct PageContext<'a> {
    registry: &'a SettingsRegistry,
    store: &'a SettingsStore,
    keymap: &'a archipel_ide_core::keymap_commands::KeymapState,
    /// Read-only IDE state for pages that show more than settings (Plugins).
    state: &'a archipel_ide_core::IdeState,
    requests: &'a mut Vec<(String, serde_json::Value)>,
}

fn draw(dialog: &mut SettingsDialog, context: PageContext<'_>, ctx: &egui::Context) -> Outcome {
    let PageContext {
        registry,
        store,
        keymap,
        state,
        requests,
    } = context;
    let mut outcome = Outcome::Keep;
    if ctx.input(|input| input.key_pressed(egui::Key::Escape))
        && !egui::Popup::is_any_open(ctx)
        && !dialog.keymap_page.capturing()
    {
        return Outcome::Cancel;
    }
    let screen = ctx.content_rect();
    // Dim and block the workbench behind the dialog.
    egui::Area::new(backdrop_id())
        .order(egui::Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(80));
        });
    let size = vec2(
        (screen.width() - 40.0).clamp(480.0, 990.0),
        (screen.height() - 40.0).clamp(360.0, 728.0),
    );
    let search = registry.search(&dialog.search);
    let searching = !dialog.search.trim().is_empty();
    egui::Area::new(egui::Id::new("settings_dialog"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.center() - size / 2.0)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let painter = ui.painter().clone();
            painter.rect(
                rect,
                8.0,
                theme::POPUP,
                egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
                egui::StrokeKind::Inside,
            );
            let title = Rect::from_min_size(rect.min, vec2(rect.width(), 34.0));
            painter.text(
                title.center(),
                egui::Align2::CENTER_CENTER,
                "Settings",
                theme::semibold(theme::UI_FONT_SIZE),
                *theme::TEXT_BRIGHT,
            );
            let mut close_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_min_size(
                        pos2(title.right() - 34.0, title.top() + 4.0),
                        vec2(28.0, 28.0),
                    ))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            if icons::button(&mut close_ui, Icon::Close, "Close").clicked() {
                outcome = Outcome::Cancel;
            }
            let body = Rect::from_min_max(
                pos2(rect.left(), title.bottom()),
                pos2(rect.right(), rect.bottom() - FOOTER_HEIGHT),
            );
            let tree_rect =
                Rect::from_min_max(body.min, pos2(body.left() + TREE_WIDTH, body.bottom()));
            let page_rect = Rect::from_min_max(pos2(tree_rect.right(), body.top()), body.max);
            painter.rect_filled(tree_rect, 0.0, theme::ISLAND);
            painter.hline(
                rect.x_range(),
                body.bottom(),
                egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
            );
            painter.vline(
                tree_rect.right(),
                body.y_range(),
                egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
            );

            let mut tree_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(tree_rect.shrink2(vec2(8.0, 8.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            tree(&mut tree_ui, dialog, registry, &search, searching);

            let mut page_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(page_rect.shrink2(vec2(18.0, 10.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            page(
                &mut page_ui,
                dialog,
                PageContext {
                    registry,
                    store,
                    keymap,
                    state,
                    requests,
                },
                &search,
                searching,
            );

            let footer = Rect::from_min_max(
                pos2(rect.left() + 16.0, body.bottom()),
                pos2(rect.right() - 16.0, rect.bottom()),
            );
            let mut footer_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(footer)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            if let Some(button) = footer_buttons(&mut footer_ui, dialog) {
                outcome = button;
            }
        });
    outcome
}

fn tree(
    ui: &mut egui::Ui,
    dialog: &mut SettingsDialog,
    registry: &SettingsRegistry,
    search: &SearchResult,
    searching: bool,
) {
    let id = egui::Id::new("settings_search");
    let response = search_field(ui, id, &mut dialog.search, "", TREE_WIDTH - 50.0);
    if std::mem::take(&mut dialog.focus_search) {
        response.request_focus();
    }
    if response.changed() {
        // Jump to the first matching page like the reference dialog does.
        let hit = registry.search(&dialog.search);
        if let Some(first) = first_hit(registry, &hit)
            && !hit.matching_pages.contains(&dialog.page)
        {
            dialog.select(registry, &first);
        }
    }
    ui.add_space(6.0);
    let modified = dialog.draft.modified_pages(registry);
    egui::ScrollArea::vertical()
        .id_salt("settings_tree")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if searching && search.visible_pages.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new("Nothing found").color(theme::TEXT_INFO));
            }
            tree_level(ui, dialog, registry, None, 0, search, searching, &modified);
        });
}

/// First matching page in tree order.
fn first_hit(registry: &SettingsRegistry, search: &SearchResult) -> Option<String> {
    fn walk(
        registry: &SettingsRegistry,
        parent: Option<&str>,
        search: &SearchResult,
    ) -> Option<String> {
        for page in registry.children(parent) {
            if search.matching_pages.contains(&page.id) {
                return Some(page.id.clone());
            }
            if let Some(hit) = walk(registry, Some(&page.id), search) {
                return Some(hit);
            }
        }
        None
    }
    walk(registry, None, search)
}

#[allow(clippy::too_many_arguments)]
fn tree_level(
    ui: &mut egui::Ui,
    dialog: &mut SettingsDialog,
    registry: &SettingsRegistry,
    parent: Option<&str>,
    depth: usize,
    search: &SearchResult,
    searching: bool,
    modified: &BTreeSet<String>,
) {
    for node in registry.children(parent) {
        if searching && !search.visible_pages.contains(&node.id) {
            continue;
        }
        let has_children = registry
            .children(Some(&node.id))
            .iter()
            .any(|child| !searching || search.visible_pages.contains(&child.id));
        let expanded = searching || dialog.expanded.contains(&node.id);
        tree_row(
            ui,
            dialog,
            registry,
            node,
            depth,
            has_children,
            expanded,
            modified,
        );
        if has_children && expanded {
            tree_level(
                ui,
                dialog,
                registry,
                Some(&node.id),
                depth + 1,
                search,
                searching,
                modified,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn tree_row(
    ui: &mut egui::Ui,
    dialog: &mut SettingsDialog,
    registry: &SettingsRegistry,
    node: &PageDef,
    depth: usize,
    has_children: bool,
    expanded: bool,
    modified: &BTreeSet<String>,
) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), TREE_ROW), Sense::click());
    let selected = dialog.page == node.id;
    let fill = if selected {
        *theme::SELECTION
    } else if response.hovered() {
        *theme::HOVER
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    let x = rect.left() + 4.0 + depth as f32 * 16.0;
    let chevron = Rect::from_min_size(pos2(x, rect.center().y - 7.0), vec2(14.0, 14.0));
    if has_children {
        icons::paint(
            ui.painter(),
            chevron,
            if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            *theme::TEXT_SECONDARY,
        );
    }
    let font = if depth == 0 {
        theme::semibold(theme::UI_FONT_SIZE)
    } else {
        egui::FontId::proportional(theme::UI_FONT_SIZE)
    };
    let color = if modified.contains(&node.id) {
        *theme::MODIFIED
    } else {
        *theme::TEXT
    };
    ui.painter().text(
        pos2(x + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &node.label,
        font,
        color,
    );
    if node.level == Scope::Project {
        // Project-level marker: a small framed square, like the reference's badge.
        let marker =
            Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(9.0, 7.0));
        ui.painter().rect_stroke(
            marker,
            1.0,
            egui::Stroke::new(1.0_f32, theme::TEXT_INFO),
            egui::StrokeKind::Inside,
        );
        ui.painter().hline(
            marker.x_range(),
            marker.top() + 2.0,
            egui::Stroke::new(1.0_f32, theme::TEXT_INFO),
        );
    }
    let chevron_hit = has_children
        && response
            .interact_pointer_pos()
            .is_some_and(|pos| chevron.expand(4.0).contains(pos));
    let response = if node.level == Scope::Project {
        response.on_hover_text("Settings for the current project")
    } else {
        response
    };
    if response.clicked() {
        if chevron_hit || (has_children && registry.page_settings(&node.id).is_empty() && selected)
        {
            toggle(&mut dialog.expanded, &node.id);
        } else {
            if has_children && !expanded {
                dialog.expanded.insert(node.id.clone());
            }
            dialog.select(registry, &node.id);
        }
    }
    if response.double_clicked() && has_children {
        toggle(&mut dialog.expanded, &node.id);
    }
}

fn toggle(set: &mut BTreeSet<String>, id: &str) {
    if !set.remove(id) {
        set.insert(id.to_owned());
    }
}

fn page(
    ui: &mut egui::Ui,
    dialog: &mut SettingsDialog,
    context: PageContext<'_>,
    search: &SearchResult,
    searching: bool,
) {
    let PageContext {
        registry,
        store,
        keymap,
        state,
        requests,
    } = context;
    let page_id = dialog.page.clone();
    // Header: breadcrumbs, Reset link, back/forward.
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        let crumbs = registry.breadcrumbs(&page_id);
        for (index, crumb) in crumbs.iter().enumerate() {
            if index > 0 {
                icons::show(ui, Icon::ChevronRight, 12.0, *theme::TEXT_SECONDARY);
            }
            ui.label(
                RichText::new(*crumb)
                    .font(theme::semibold(theme::UI_FONT_SIZE))
                    .color(theme::TEXT_BRIGHT),
            );
        }
        if registry
            .page(&page_id)
            .is_some_and(|p| p.level == Scope::Project)
        {
            ui.label(RichText::new("For current project").color(theme::TEXT_INFO));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let forward = ui.add(
                archipel_kit::action_system::ActionButton::new(
                    archipel_kit::Glyph::ArrowRight,
                    "Forward",
                )
                .enabled(!dialog.forward.is_empty()),
            );
            let back = ui.add(
                archipel_kit::action_system::ActionButton::new(archipel_kit::Glyph::ArrowLeft, "Back")
                    .enabled(!dialog.back.is_empty()),
            );
            if forward.clicked() {
                dialog.navigate(false);
            }
            if back.clicked() {
                dialog.navigate(true);
            }
            let page_modified = registry
                .page_settings(&page_id)
                .iter()
                .any(|def| dialog.draft.is_setting_modified(&def.id));
            if page_modified && ui.link("Reset").clicked() {
                dialog.draft.reset_page(registry, &page_id);
            }
        });
    });
    ui.add_space(6.0);
    if page_id == "preferences.keymap" {
        keymap_page(ui, dialog, registry, store, keymap, requests);
        return;
    }
    if page_id == "editing.templates" {
        egui::ScrollArea::vertical()
            .id_salt("live_templates_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                crate::settings_live_templates::show(
                    ui,
                    &mut dialog.live_templates_page,
                    &state.live_templates(),
                    requests,
                );
            });
        return;
    }
    if page_id == "preferences.pluginManager" {
        crate::plugin_manager::page(
            ui,
            &mut dialog.plugins_page,
            &mut crate::plugin_manager::PageContext { state, requests },
        );
        return;
    }
    let settings = registry.page_settings(&page_id);
    egui::ScrollArea::vertical()
        .id_salt(("settings_page", &page_id))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            match page_id.as_str() {
                crate::settings_pages::FILE_TEMPLATES_PAGE => {
                    crate::settings_pages::file_templates(ui);
                    return;
                }
                crate::settings_pages::VCS_PAGE => {
                    crate::settings_pages::version_control(ui, state);
                    ui.add_space(12.0);
                    for child in registry.children(Some(&page_id)) {
                        if ui.link(&child.label).clicked() {
                            dialog.select(registry, &child.id);
                        }
                    }
                    return;
                }
                _ => {}
            }
            if settings.is_empty() {
                empty_page(ui, dialog, registry, &page_id);
                return;
            }
            let mut group: Option<&str> = None;
            let mut first = true;
            for def in settings {
                if def.group.as_deref() != group || first {
                    group = def.group.as_deref();
                    if let Some(heading) = group {
                        group_heading(ui, heading, first);
                    }
                    first = false;
                }
                let highlight = searching && search.matching_settings.contains(&def.id);
                setting_row(ui, dialog, registry, store, def, group.is_some(), highlight);
            }
            if page_id == "AppEditorFontConfigurable" {
                font_preview(ui, dialog, registry, store);
            }
            ui.add_space(12.0);
        });
}

/// Editor > Font: sample text at the size and line height being edited.
fn font_preview(
    ui: &mut egui::Ui,
    dialog: &SettingsDialog,
    registry: &SettingsRegistry,
    store: &SettingsStore,
) {
    use archipel_ide_settings::ids;
    let size = dialog
        .draft
        .value(registry, store, ids::EDITOR_FONT_SIZE)
        .ok()
        .and_then(|value| value.as_int())
        .unwrap_or(13)
        .clamp(4, 72) as f32;
    let line_height = dialog
        .draft
        .value(registry, store, ids::EDITOR_LINE_HEIGHT)
        .ok()
        .and_then(|value| value.as_float())
        .unwrap_or(1.2)
        .clamp(0.6, 3.0) as f32;
    group_heading(ui, "Font preview", false);
    egui::Frame::new()
        .fill(*theme::editor::BACKGROUND)
        .stroke(egui::Stroke::new(1.0_f32, theme::POPUP_BORDER))
        .corner_radius(4)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = size * (line_height - 1.0).max(0.0);
            let font = egui::FontId::monospace(size);
            for line in [
                "The quick brown fox jumps over the lazy dog",
                "0123456789 () {} [] <> == != -> =>",
                "public static void main(String[] args)",
            ] {
                ui.label(
                    RichText::new(line)
                        .font(font.clone())
                        .color(*theme::editor::TEXT),
                );
            }
        });
}

/// Keymap: the selector is the generic `keymap.active` row; the rest is custom.
fn keymap_page(
    ui: &mut egui::Ui,
    dialog: &mut SettingsDialog,
    registry: &SettingsRegistry,
    store: &SettingsStore,
    keymap: &archipel_ide_core::keymap_commands::KeymapState,
    requests: &mut Vec<(String, serde_json::Value)>,
) {
    use archipel_ide_settings::ids;
    if let Some(def) = registry.setting(ids::KEYMAP) {
        setting_row(ui, dialog, registry, store, def, false, false);
    }
    let selected = dialog
        .draft
        .value(registry, store, ids::KEYMAP)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| keymap.keymap.id.clone());
    crate::settings_keymap::show(ui, &mut dialog.keymap_page, keymap, &selected, requests);
}

fn empty_page(
    ui: &mut egui::Ui,
    dialog: &mut SettingsDialog,
    registry: &SettingsRegistry,
    page: &str,
) {
    let children = registry.children(Some(page));
    if children.is_empty() {
        ui.add_space(12.0);
        ui.label(
            RichText::new("This page has no options in Archipel IDE yet.")
                .color(theme::TEXT_SECONDARY),
        );
        ui.label(
            RichText::new(
                "It exists in the reference IDE; see docs/parity/ for the implementation status.",
            )
            .color(theme::TEXT_INFO),
        );
        return;
    }
    ui.add_space(4.0);
    for child in children {
        if ui.link(&child.label).clicked() {
            dialog.select(registry, &child.id);
        }
        ui.add_space(2.0);
    }
}

fn group_heading(ui: &mut egui::Ui, heading: &str, first: bool) {
    ui.add_space(if first { 4.0 } else { 18.0 });
    ui.horizontal(|ui| {
        ui.label(RichText::new(heading).color(theme::TEXT_BRIGHT));
        let y = ui.min_rect().center().y;
        let left = ui.cursor().left() + 4.0;
        let right = ui.max_rect().right();
        if right > left {
            ui.painter().hline(
                left..=right,
                y,
                egui::Stroke::new(1.0_f32, theme::POPUP_BORDER),
            );
        }
    });
    ui.add_space(4.0);
}

fn setting_row(
    ui: &mut egui::Ui,
    dialog: &mut SettingsDialog,
    registry: &SettingsRegistry,
    store: &SettingsStore,
    def: &SettingDef,
    indented: bool,
    highlight: bool,
) {
    let Ok(current) = dialog.draft.value(registry, store, &def.id) else {
        return;
    };
    let mut next = None;
    let indent = if indented { 18.0 } else { 0.0 };
    ui.horizontal(|ui| {
        ui.add_space(indent);
        let label_color = *theme::TEXT;
        match (&def.kind, &current) {
            (SettingKind::Bool, SettingValue::Bool(value)) => {
                let mut checked = *value;
                let response =
                    ui.checkbox(&mut checked, RichText::new(&def.label).color(label_color));
                mark(ui, response.rect, highlight);
                if response.changed() {
                    next = Some(SettingValue::Bool(checked));
                }
            }
            (kind, value) => {
                let response =
                    ui.label(RichText::new(format!("{}:", def.label)).color(label_color));
                mark(ui, response.rect, highlight);
                next = value_editor(ui, &def.id, kind, value);
            }
        }
        if def.requires_restart {
            ui.label(RichText::new("Requires restart").color(theme::TEXT_INFO));
        }
    });
    if let Some(description) = &def.description {
        ui.horizontal(|ui| {
            ui.add_space(
                indent
                    + if matches!(def.kind, SettingKind::Bool) {
                        24.0
                    } else {
                        0.0
                    },
            );
            ui.add(
                egui::Label::new(
                    RichText::new(description)
                        .color(theme::TEXT_INFO)
                        .size(theme::UI_FONT_SIZE - 1.0),
                )
                .wrap(),
            );
        });
    }
    ui.add_space(4.0);
    if let Some(value) = next
        && let Err(error) = dialog.draft.set(registry, store, &def.id, value)
    {
        dialog.error = Some(error.to_string());
    }
}

/// Search hits are highlighted like the reference dialog marks matching options.
fn mark(ui: &egui::Ui, rect: Rect, highlight: bool) {
    if highlight {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            3.0,
            egui::Stroke::new(1.0_f32, theme::WARNING),
            egui::StrokeKind::Outside,
        );
    }
}

fn value_editor(
    ui: &mut egui::Ui,
    id: &str,
    kind: &SettingKind,
    value: &SettingValue,
) -> Option<SettingValue> {
    match (kind, value) {
        (SettingKind::Enum { options }, SettingValue::Text(selected)) => {
            let label = options
                .iter()
                .find(|option| &option.id == selected)
                .map(|option| option.label.as_str())
                .unwrap_or(selected);
            let mut choice = None;
            egui::ComboBox::from_id_salt(("setting", id))
                .selected_text(label)
                .width(200.0)
                .show_ui(ui, |ui| {
                    for option in options {
                        if ui
                            .selectable_label(&option.id == selected, &option.label)
                            .clicked()
                        {
                            choice = Some(SettingValue::Text(option.id.clone()));
                        }
                    }
                });
            choice
        }
        (SettingKind::Int { min, max }, SettingValue::Int(number)) => {
            number_field(ui, id, &number.to_string(), |text| {
                text.trim()
                    .parse::<i64>()
                    .ok()
                    .filter(|value| (*min..=*max).contains(value))
                    .map(SettingValue::Int)
            })
        }
        (SettingKind::Float { min, max }, value) => {
            let current = value.as_float().unwrap_or(*min);
            number_field(ui, id, &current.to_string(), |text| {
                text.trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite() && (*min..=*max).contains(value))
                    .map(SettingValue::Float)
            })
        }
        (SettingKind::Text | SettingKind::Path, SettingValue::Text(text)) => {
            let mut edited = text.clone();
            ui.add(egui::TextEdit::singleline(&mut edited).desired_width(320.0))
                .changed()
                .then_some(SettingValue::Text(edited))
        }
        (SettingKind::List, SettingValue::List(items)) => {
            let mut edited = items.join("\n");
            ui.add(
                egui::TextEdit::multiline(&mut edited)
                    .desired_rows(3)
                    .desired_width(320.0),
            )
            .changed()
            .then(|| SettingValue::List(edited.lines().map(Into::into).collect()))
        }
        _ => {
            ui.label(RichText::new("unsupported value").color(theme::ERROR));
            None
        }
    }
}

/// Text field for numbers. While focused it keeps the typed text (which may be
/// temporarily invalid, shown with a red border); valid input becomes an edit.
fn number_field(
    ui: &mut egui::Ui,
    id: &str,
    current: &str,
    parse: impl Fn(&str) -> Option<SettingValue>,
) -> Option<SettingValue> {
    let key = egui::Id::new(("setting_number", id));
    let mut text = ui
        .data(|data| data.get_temp::<String>(key))
        .unwrap_or_else(|| current.to_owned());
    let parsed = parse(&text);
    let mut edit = egui::TextEdit::singleline(&mut text)
        .id(key)
        .desired_width(64.0);
    if parsed.is_none() {
        edit = edit.text_color(*theme::ERROR);
    }
    let response = ui.add(edit);
    if response.has_focus() {
        ui.data_mut(|data| data.insert_temp(key, text.clone()));
    } else {
        ui.data_mut(|data| data.remove::<String>(key));
    }
    if response.changed() {
        parse(&text)
    } else {
        None
    }
}

fn footer_buttons(ui: &mut egui::Ui, dialog: &mut SettingsDialog) -> Option<Outcome> {
    let mut outcome = None;
    ui.spacing_mut().item_spacing.x = 10.0;
    let size = vec2(72.0, 28.0);
    let modified = dialog.draft.is_modified();
    if ui
        .add_enabled(modified, egui::Button::new("Apply").min_size(size))
        .clicked()
    {
        outcome = Some(Outcome::Apply);
    }
    if ui.add(egui::Button::new("Cancel").min_size(size)).clicked() {
        outcome = Some(Outcome::Cancel);
    }
    let ok = egui::Button::new(RichText::new("OK").color(Color32::WHITE))
        .fill(theme::ACCENT)
        .min_size(size);
    if ui.add(ok).clicked() {
        outcome = Some(Outcome::Ok);
    }
    if let Some(error) = &dialog.error {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(RichText::new(error).color(theme::ERROR));
        });
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selecting_a_page_expands_ancestors_and_records_history() {
        let registry = SettingsRegistry::intellij();
        let mut dialog = SettingsDialog::new(&registry, "preferences.lookFeel");
        assert!(dialog.expanded.contains("appearance"));
        assert!(dialog.back.is_empty());
        dialog.select(&registry, "reference.settingsdialog.project.gradle");
        assert!(dialog.expanded.contains("build"));
        assert!(dialog.expanded.contains("build.tools"));
        dialog.navigate(true);
        assert_eq!(dialog.page, "preferences.lookFeel");
        dialog.navigate(false);
        assert_eq!(dialog.page, "reference.settingsdialog.project.gradle");
        // Unknown pages are ignored.
        dialog.select(&registry, "no.such.page");
        assert_eq!(dialog.page, "reference.settingsdialog.project.gradle");
    }

    #[test]
    fn first_search_hit_follows_tree_order() {
        let registry = SettingsRegistry::intellij();
        let hit = first_hit(&registry, &registry.search("smooth"));
        assert_eq!(hit.as_deref(), Some("preferences.lookFeel"));
        assert_eq!(first_hit(&registry, &registry.search("zzzz")), None);
    }
}
