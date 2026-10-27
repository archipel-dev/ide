//! Merged Manifest view: merged lines tinted by the manifest they come from,
//! with the "Manifest Sources" legend; clicking a project line opens it.

use crate::{IdeApp, theme};
use archipel_ide_android::merged_manifest::ManifestSource;
use eframe::egui::{self, Color32, RichText, vec2};
use serde_json::json;

fn tint(index: usize) -> Color32 {
    const TINTS: [Color32; 6] = [
        Color32::from_rgba_premultiplied(0x23, 0x2C, 0x3D, 0xFF),
        Color32::from_rgba_premultiplied(0x2E, 0x34, 0x26, 0xFF),
        Color32::from_rgba_premultiplied(0x3A, 0x2B, 0x2E, 0xFF),
        Color32::from_rgba_premultiplied(0x2A, 0x36, 0x38, 0xFF),
        Color32::from_rgba_premultiplied(0x38, 0x33, 0x24, 0xFF),
        Color32::from_rgba_premultiplied(0x33, 0x2A, 0x3A, 0xFF),
    ];
    TINTS[index % TINTS.len()]
}

/// Whether the active editor file is an Android manifest that gets the
/// `Text | Merged Manifest` tab bar.
pub fn applies(app: &IdeApp) -> bool {
    app.state.is_android_project()
        && app.state.active_doc().is_some_and(|d| {
            d.path
                .file_name()
                .is_some_and(|n| n == "AndroidManifest.xml")
        })
}

/// Bottom tab bar of the manifest editor.
pub fn tab_bar(app: &mut IdeApp, ui: &mut egui::Ui) {
    let merged = app.state.android().merged_manifest.open;
    let mut choice = None;
    egui::TopBottomPanel::bottom("manifest_tabs")
        .exact_height(26.0)
        .frame(egui::Frame::new())
        .show_inside(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                for (is_merged, label) in [(false, "Text"), (true, "Merged Manifest")] {
                    let selected = merged == is_merged;
                    let text = RichText::new(label).color(if selected {
                        *theme::TEXT_BRIGHT
                    } else {
                        *theme::TEXT_SECONDARY
                    });
                    if ui
                        .add(egui::Button::new(text).selected(selected).frame(selected))
                        .clicked()
                        && !selected
                    {
                        choice = Some(is_merged);
                    }
                }
            });
        });
    match choice {
        Some(true) => app.dispatch("android.manifest.merged", serde_json::Value::Null),
        Some(false) => app.dispatch("android.manifest.text", serde_json::Value::Null),
        None => {}
    }
}

/// The merged tab's body, drawn in place of the text editor.
pub fn body(app: &mut IdeApp, ui: &mut egui::Ui) {
    let root = app
        .state
        .project
        .as_ref()
        .map(|p| p.root().to_owned())
        .unwrap_or_default();
    let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let mut open: Option<(std::path::PathBuf, usize)> = None;
    let mut build: Option<String> = None;
    let view = &app.state.android().merged_manifest;
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(&view.title)
                .font(theme::semibold(theme::UI_FONT_SIZE))
                .color(theme::TEXT_BRIGHT),
        );
    });
    ui.add_space(6.0);
    let merged = match &view.result {
        Some(Ok(merged)) => merged,
        Some(Err(error)) => {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(error).color(theme::TEXT_INFO));
                    if let Some(task) = &view.build_task
                        && ui.button(format!("Build ({task})")).clicked()
                    {
                        build = Some(task.clone());
                    }
                });
            });
            if let Some(task) = build {
                app.dispatch("gradle.runTask", json!({"task": task}));
            }
            if close {
                app.dispatch("android.manifest.text", serde_json::Value::Null);
            }
            return;
        }
        None => return,
    };
    {
        let legend_width = 360.0;
        {
            ui.horizontal_top(|ui| {
                let font = egui::FontId::monospace(theme::EDITOR_FONT_SIZE);
                let row = ui.fonts_mut(|f| f.row_height(&font)) + 2.0;
                let code_width = ui.available_width() - legend_width - 16.0;
                ui.vertical(|ui| {
                    ui.set_width(code_width);
                    ui.set_height(ui.available_height());
                    egui::ScrollArea::both()
                        .id_salt("merged_manifest_lines")
                        .auto_shrink([false, false])
                        .show_rows(ui, row, merged.lines.len(), |ui, range| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for line in &merged.lines[range] {
                                let (rect, response) = ui.allocate_exact_size(
                                    vec2(ui.available_width().max(code_width), row),
                                    egui::Sense::click(),
                                );
                                ui.painter().rect_filled(rect, 0.0, tint(line.source));
                                ui.painter().text(
                                    rect.left_center() + vec2(6.0, 0.0),
                                    egui::Align2::LEFT_CENTER,
                                    &line.text,
                                    font.clone(),
                                    *theme::TEXT,
                                );
                                let source = &merged.sources[line.source];
                                let response = response.on_hover_text(source.label(&root));
                                if response.clicked()
                                    && let (ManifestSource::File { path, .. }, Some(at)) =
                                        (source, line.source_line)
                                {
                                    open = Some((path.clone(), at));
                                }
                            }
                        });
                });
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.set_width(legend_width);
                    ui.label(
                        RichText::new("Manifest Sources")
                            .font(theme::semibold(theme::UI_FONT_SIZE))
                            .underline()
                            .color(theme::TEXT_BRIGHT),
                    );
                    ui.add_space(6.0);
                    for (index, source) in merged.sources.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let (swatch, _) =
                                ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::hover());
                            ui.painter().rect_filled(swatch, 2.0, tint(index));
                            ui.painter().rect_stroke(
                                swatch,
                                2.0,
                                egui::Stroke::new(1.0_f32, theme::BORDER),
                                egui::StrokeKind::Inside,
                            );
                            ui.label(RichText::new(source.label(&root)).color(theme::MODIFIED));
                        });
                    }
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!("Blame report: {}", merged.blame_file.display()))
                            .color(theme::TEXT_INFO),
                    );
                });
            });
        }
    }
    if let Some((path, line)) = open {
        let relative = path
            .strip_prefix(&root)
            .map(|p| p.to_owned())
            .unwrap_or(path);
        app.dispatch("editor.openFile", json!({"path": relative}));
        app.dispatch("editor.gotoLine", json!({"line": line}));
        close = true;
    }
    if close {
        app.dispatch("android.manifest.text", serde_json::Value::Null);
    }
}
