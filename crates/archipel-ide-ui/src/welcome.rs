//! Welcome window for launches without a project: navigation column, recent
//! projects with search, and New Project / Open / Get from VCS buttons.

use crate::icons::{self, Icon};
use crate::{IdeApp, chrome, theme, widgets};
use eframe::egui::{self, Align2, Color32, RichText, Sense, vec2};
use serde_json::json;
use std::path::PathBuf;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    #[default]
    Projects,
    Customize,
    Plugins,
    Learn,
}

impl Page {
    const ALL: [(Page, &'static str); 4] = [
        (Page::Projects, "Projects"),
        (Page::Customize, "Customize"),
        (Page::Plugins, "Plugins"),
        (Page::Learn, "Learn"),
    ];
}

#[derive(Default)]
pub struct State {
    pub page: Page,
    pub search: String,
}

/// Whether the welcome window replaces the IDE chrome.
pub fn active(app: &IdeApp) -> bool {
    app.state.project.is_none() && app.state.plugin_host.app_mode.is_none()
}

pub fn show(app: &mut IdeApp, ctx: &egui::Context) {
    title_strip(ctx);
    let mut open: Option<PathBuf> = None;
    let mut remove: Option<PathBuf> = None;
    egui::SidePanel::left("welcome_nav")
        .exact_width(230.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(widgets::chrome_frame(ctx).inner_margin(egui::Margin::symmetric(12, 8)))
        .show(ctx, |ui| nav_column(app, ui));
    egui::CentralPanel::default()
        .frame(widgets::island_frame(ctx).inner_margin(32))
        .show(ctx, |ui| match app.welcome.page {
            Page::Projects => projects_page(app, ui, &mut open, &mut remove),
            Page::Customize => info_page(
                app,
                ui,
                "Customize",
                "Theme, editor font, keymap and other preferences.",
                "Open Settings…",
                json!({"page": "appearance"}),
            ),
            Page::Plugins => info_page(
                app,
                ui,
                "Plugins",
                "Install, enable and disable plugins.",
                "Manage Plugins…",
                json!({"page": "preferences.pluginManager"}),
            ),
            Page::Learn => learn_page(ui),
        });
    if let Some(path) = remove {
        app.dispatch("project.removeRecent", json!({"path": path}));
    }
    if let Some(path) = open
        && let Err(message) = app.open_project_path(&path.to_string_lossy(), ctx)
    {
        app.state.status = message;
    }
}

/// Drag area with window controls; the welcome window has no toolbar.
fn title_strip(ctx: &egui::Context) {
    egui::TopBottomPanel::top("welcome_title")
        .exact_height(chrome::TOOLBAR_HEIGHT)
        .show_separator_line(false)
        .frame(widgets::chrome_frame(ctx))
        .show(ctx, |ui| {
            let bar = ui.max_rect();
            let drag = ui.interact(bar, egui::Id::new("welcome_drag"), Sense::click_and_drag());
            if drag.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            if chrome::custom_title_bar() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    chrome::window_controls(ui);
                });
            }
        });
}

fn nav_column(app: &mut IdeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, 8.0, theme::avatar_color("Archipel IDE"));
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "JT",
            theme::semibold(16.0),
            Color32::WHITE,
        );
        ui.vertical(|ui| {
            ui.add_space(2.0);
            ui.label(
                RichText::new("Archipel IDE")
                    .font(theme::semibold(15.0))
                    .color(theme::TEXT_BRIGHT),
            );
            ui.label(
                RichText::new(env!("CARGO_PKG_VERSION"))
                    .size(11.0)
                    .color(theme::TEXT_SECONDARY),
            );
        });
    });
    ui.add_space(16.0);
    let plugins = app
        .state
        .plugin_host
        .registry
        .entries()
        .iter()
        .filter(|entry| entry.is_active())
        .count();
    for (page, label) in Page::ALL {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
        let selected = app.welcome.page == page;
        if selected || response.hovered() {
            let fill = if selected {
                *theme::SELECTION
            } else {
                *theme::HOVER
            };
            ui.painter().rect_filled(rect, 6.0, fill);
        }
        ui.painter().text(
            rect.left_center() + vec2(12.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(theme::UI_FONT_SIZE + 1.0),
            *theme::TEXT,
        );
        if page == Page::Plugins && plugins > 0 {
            ui.painter().text(
                rect.right_center() - vec2(12.0, 0.0),
                Align2::RIGHT_CENTER,
                plugins.to_string(),
                egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
                *theme::TEXT_SECONDARY,
            );
        }
        if response.clicked() {
            app.welcome.page = page;
        }
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(4.0);
        let settings = icons::button(ui, Icon::Task, "Settings");
        if settings.clicked() {
            app.dispatch("settings.open", serde_json::Value::Null);
        }
    });
}

fn projects_page(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    open: &mut Option<PathBuf>,
    remove: &mut Option<PathBuf>,
) {
    ui.label(
        RichText::new("Welcome to Archipel IDE")
            .font(theme::semibold(20.0))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(4.0);
    ui.label(RichText::new("Create a new project from scratch.").color(theme::TEXT_SECONDARY));
    ui.label(
        RichText::new("Open an existing project from disk or version control.")
            .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        if action_button(ui, Icon::Plus, "New Project").clicked() {
            app.new_project_dialog();
        }
        if action_button(ui, Icon::Folder, "Open").clicked() {
            app.open_project_dialog();
        }
        if action_button(ui, Icon::Branch, "Get from VCS").clicked() {
            app.clone_dialog();
        }
    });
    ui.add_space(20.0);
    if app.state.recent_projects.entries().is_empty() {
        ui.label(
            RichText::new(format!(
                "No recent projects. Open Project: {}",
                crate::keys::display("⌘O")
            ))
            .color(theme::TEXT_INFO),
        );
    } else {
        widgets::search_field(
            ui,
            egui::Id::new("welcome_search"),
            &mut app.welcome.search,
            "Search projects",
            ui.available_width() - 40.0,
        );
        ui.add_space(8.0);
        let matches: Vec<(PathBuf, String)> = app
            .state
            .recent_projects
            .search(&app.welcome.search)
            .into_iter()
            .map(|entry| (entry.path.clone(), entry.name.clone()))
            .collect();
        if matches.is_empty() {
            ui.label(RichText::new("No matching projects.").color(theme::TEXT_SECONDARY));
        }
        egui::ScrollArea::vertical()
            .id_salt("welcome_recent")
            .show(ui, |ui| {
                for (path, name) in &matches {
                    recent_row(ui, path, name, open, remove);
                }
            });
    }
    if !app.state.status.is_empty() {
        ui.add_space(12.0);
        ui.label(RichText::new(&app.state.status).color(theme::TEXT_SECONDARY));
    }
}

fn action_button(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(110.0, 84.0), Sense::click());
    let fill = if response.hovered() {
        *theme::HOVER
    } else {
        *theme::FIELD
    };
    ui.painter().rect_filled(rect, 8.0, fill);
    ui.painter().rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0_f32, theme::BORDER),
        egui::StrokeKind::Inside,
    );
    let icon_rect = egui::Rect::from_center_size(rect.center() - vec2(0.0, 14.0), vec2(24.0, 24.0));
    icons::paint(ui.painter(), icon_rect, icon, *theme::TEXT);
    ui.painter().text(
        rect.center() + vec2(0.0, 24.0),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT,
    );
    // The tile already shows its label; a tooltip repeating it is noise.
    response
}

fn recent_row(
    ui: &mut egui::Ui,
    path: &PathBuf,
    name: &str,
    open: &mut Option<PathBuf>,
    remove: &mut Option<PathBuf>,
) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, *theme::HOVER);
    }
    let badge = egui::Rect::from_min_size(rect.min + vec2(8.0, 6.0), vec2(36.0, 36.0));
    ui.painter()
        .rect_filled(badge, 8.0, theme::avatar_color(name));
    ui.painter().text(
        badge.center(),
        Align2::CENTER_CENTER,
        theme::project_initials(name),
        theme::semibold(13.0),
        Color32::WHITE,
    );
    ui.painter().text(
        rect.min + vec2(56.0, 14.0),
        Align2::LEFT_CENTER,
        name,
        theme::semibold(theme::UI_FONT_SIZE + 1.0),
        *theme::TEXT_BRIGHT,
    );
    ui.painter().text(
        rect.min + vec2(56.0, 33.0),
        Align2::LEFT_CENTER,
        path.display().to_string(),
        egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
        *theme::TEXT_SECONDARY,
    );
    let close = egui::Rect::from_center_size(
        egui::pos2(rect.right() - 20.0, rect.center().y),
        vec2(22.0, 22.0),
    );
    let close_response = ui.interact(
        close,
        egui::Id::new(("welcome_remove", path)),
        Sense::click(),
    );
    if response.hovered() || close_response.hovered() {
        icons::paint(
            ui.painter(),
            close.shrink(4.0),
            Icon::Close,
            *theme::TEXT_SECONDARY,
        );
    }
    if close_response
        .on_hover_text("Remove from Recent Projects")
        .clicked()
    {
        *remove = Some(path.clone());
    } else if response.clicked() {
        *open = Some(path.clone());
    }
}

fn info_page(
    app: &mut IdeApp,
    ui: &mut egui::Ui,
    heading: &str,
    text: &str,
    button: &str,
    params: serde_json::Value,
) {
    ui.label(
        RichText::new(heading)
            .font(theme::semibold(20.0))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(6.0);
    ui.label(RichText::new(text).color(theme::TEXT_SECONDARY));
    ui.add_space(14.0);
    if ui.button(button).clicked() {
        app.dispatch("settings.open", params);
    }
}

fn learn_page(ui: &mut egui::Ui) {
    ui.label(
        RichText::new("Learn")
            .font(theme::semibold(20.0))
            .color(theme::TEXT_BRIGHT),
    );
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Press Shift twice to search everywhere, or use Find Action to run any command.",
        )
        .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(10.0);
    ui.hyperlink_to(
        "Archipel IDE on GitHub",
        "https://github.com/archipel-dev/ide",
    );
}
