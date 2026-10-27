//! Resource Manager tool window: module combo, resource type tabs, search and
//! a grid of previews (raster images, rendered vector drawables, colours,
//! string values). Double-clicking an entry opens its declaration.

use crate::{IdeApp, theme, widgets};
use archipel_ide_android::resource_manager::{Preview, ResourceItem, ResourceTab};
use archipel_ide_android::vector::VectorDrawable;
use eframe::egui::{self, Color32, RichText, Sense, TextureHandle, vec2};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const CELL_WIDTH: f32 = 112.0;
const PREVIEW_SIZE: f32 = 72.0;

type Textures = Arc<Mutex<HashMap<PathBuf, Option<TextureHandle>>>>;

fn cache_id() -> egui::Id {
    egui::Id::new("resource_manager_textures")
}

/// Drops cached previews so edited files are re-read.
pub fn clear_cache(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<Textures>(cache_id()));
}

fn load_png(ctx: &egui::Context, root: &Path, path: &Path) -> Option<TextureHandle> {
    let bytes = std::fs::read(root.join(path)).ok()?;
    let image = image::load_from_memory(&bytes)
        .ok()?
        .thumbnail(256, 256)
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    Some(ctx.load_texture(path.to_string_lossy(), color, egui::TextureOptions::LINEAR))
}

fn load_vector(
    ctx: &egui::Context,
    root: &Path,
    path: &Path,
    colors: &HashMap<String, u32>,
) -> Option<TextureHandle> {
    let xml = std::fs::read_to_string(root.join(path)).ok()?;
    let resolve = |reference: &str| colors.get(reference.trim_start_matches("@color/")).copied();
    let drawable = VectorDrawable::parse(&xml, &resolve).ok()?;
    let image = drawable.render(PREVIEW_SIZE as u32 * 2);
    let size = [image.width as usize, image.height as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &image.pixels);
    Some(ctx.load_texture(path.to_string_lossy(), color, egui::TextureOptions::LINEAR))
}

fn argb(color: u32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color >> 16) as u8,
        (color >> 8) as u8,
        color as u8,
        (color >> 24) as u8,
    )
}

fn module_label(module: &Path) -> String {
    let name = module.to_string_lossy();
    if name.is_empty() {
        "<root>".into()
    } else {
        name.into_owned()
    }
}

pub fn show(app: &mut IdeApp, ui: &mut egui::Ui) {
    if !app.state.is_android_project() {
        ui.add_space(12.0);
        ui.label(RichText::new("Not an Android project").color(theme::TEXT_INFO));
        return;
    }
    let modules = app.state.android_module_dirs();
    let tooling = app.state.android_tooling();
    let manager = tooling.resource_manager();
    let module = manager.module.clone().or_else(|| modules.first().cloned());
    let tab = manager.tab;
    let mut query = manager.query.clone();
    drop(manager);
    let mut select_module = None;
    let mut select_tab = None;

    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Module:").color(theme::TEXT_SECONDARY));
                egui::ComboBox::from_id_salt("resource_manager_module")
                    .selected_text(module.as_deref().map(module_label).unwrap_or_default())
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for dir in &modules {
                            let selected = module.as_ref() == Some(dir);
                            if ui.selectable_label(selected, module_label(dir)).clicked() {
                                select_module = Some(dir.clone());
                            }
                        }
                    });
            });
            ui.add_space(4.0);
            let width = ui.available_width() - 30.0;
            widgets::search_field(
                ui,
                egui::Id::new("resource_manager_search"),
                &mut query,
                "Search resources",
                width.max(80.0),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.spacing_mut().button_padding = vec2(4.0, 2.0);
                for candidate in ResourceTab::ALL {
                    let selected = candidate == tab;
                    let text = RichText::new(candidate.title()).color(if selected {
                        *theme::TEXT_BRIGHT
                    } else {
                        *theme::TEXT_SECONDARY
                    });
                    if ui
                        .add(egui::Button::new(text).selected(selected).frame(selected))
                        .clicked()
                    {
                        select_tab = Some(candidate);
                    }
                }
            });
        });
    ui.separator();

    let mut params = serde_json::Map::new();
    if let Some(dir) = select_module {
        params.insert("module".into(), json!(dir));
    }
    if let Some(candidate) = select_tab {
        params.insert("tab".into(), json!(candidate.id()));
    }
    if query != app.state.android_tooling().resource_manager().query {
        params.insert("query".into(), json!(query));
    }
    if !params.is_empty() {
        app.dispatch("android.resourceManager.select", params.into());
    }

    let items: Vec<ResourceItem> = app
        .state
        .android_tooling()
        .resource_manager()
        .visible(&modules)
        .into_iter()
        .cloned()
        .collect();
    if items.is_empty() {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(format!("No {} resources", tab.title().to_lowercase()))
                    .color(theme::TEXT_INFO),
            );
        });
        return;
    }
    let Some(root) = app.state.project_root() else {
        return;
    };
    let colors: HashMap<String, u32> = app
        .state
        .android_tooling()
        .resource_manager()
        .items
        .iter()
        .filter_map(|i| match i.preview {
            Preview::Color(c) if i.tab == ResourceTab::Color => Some((i.name.clone(), c)),
            _ => None,
        })
        .collect();
    let textures: Textures = ui
        .ctx()
        .data_mut(|d| d.get_temp::<Textures>(cache_id()))
        .unwrap_or_else(|| {
            let fresh = Textures::default();
            ui.ctx()
                .data_mut(|d| d.insert_temp(cache_id(), fresh.clone()));
            fresh
        });
    let columns = ((ui.available_width() / CELL_WIDTH) as usize).max(1);
    let mut open = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(6.0);
            egui::Grid::new("resource_grid")
                .spacing([6.0, 10.0])
                .show(ui, |ui| {
                    for (n, item) in items.iter().enumerate() {
                        let response = cell(ui, item, &root, &textures, &colors);
                        if response.double_clicked() {
                            open = Some((item.path.clone(), item.line));
                        }
                        if (n + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
    if let Some((path, line)) = open {
        app.dispatch(
            "android.resourceManager.open",
            json!({"path": path, "line": line}),
        );
    }
}

fn cell(
    ui: &mut egui::Ui,
    item: &ResourceItem,
    root: &Path,
    textures: &Textures,
    colors: &HashMap<String, u32>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(CELL_WIDTH - 6.0, 112.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, *theme::SELECTION);
    }
    let preview = egui::Rect::from_center_size(
        rect.center_top() + vec2(0.0, 6.0 + PREVIEW_SIZE / 2.0),
        vec2(PREVIEW_SIZE, PREVIEW_SIZE),
    );
    ui.painter().rect_filled(preview, 4.0, *theme::FIELD);
    match &item.preview {
        Preview::Color(color) => {
            ui.painter()
                .rect_filled(preview.shrink(8.0), 4.0, argb(*color));
        }
        Preview::Text(text) => {
            let shown: String = text.chars().take(40).collect();
            ui.painter().text(
                preview.shrink(4.0).left_top(),
                egui::Align2::LEFT_TOP,
                shown,
                egui::FontId::proportional(11.0),
                *theme::TEXT,
            );
        }
        Preview::Image(path) | Preview::Vector(path) => {
            let texture = textures.lock().ok().and_then(|mut cache| {
                cache
                    .entry(path.clone())
                    .or_insert_with(|| {
                        if matches!(item.preview, Preview::Vector(_)) {
                            load_vector(ui.ctx(), root, path, colors)
                        } else {
                            load_png(ui.ctx(), root, path)
                        }
                    })
                    .clone()
            });
            if let Some(texture) = texture {
                let size = texture.size_vec2();
                let scale = (PREVIEW_SIZE / size.x.max(size.y)).min(1.0e3);
                let fitted = egui::Rect::from_center_size(preview.center(), size * scale);
                ui.painter().image(
                    texture.id(),
                    fitted,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        Preview::None => {}
    }
    let label_top = preview.bottom() + 4.0;
    ui.painter().text(
        egui::pos2(rect.center().x, label_top),
        egui::Align2::CENTER_TOP,
        elide(&item.name, 16),
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        *theme::TEXT_BRIGHT,
    );
    let versions = widgets::pluralize(item.versions, "version", "versions");
    ui.painter().text(
        egui::pos2(rect.center().x, label_top + 15.0),
        egui::Align2::CENTER_TOP,
        versions,
        egui::FontId::proportional(11.0),
        *theme::TEXT_INFO,
    );
    response.on_hover_text(item.path.to_string_lossy())
}

fn elide(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        return name.to_owned();
    }
    let head: String = name.chars().take(max.saturating_sub(1)).collect();
    format!("{head}…")
}
