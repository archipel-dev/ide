//! Running Devices tool window: the empty state with links to Device Manager,
//! and the mirrored screen of the selected device when one is connected.

use crate::{IdeApp, icons, theme, widgets};
use archipel_ide_core::{Anchor, ToolWindow};
use eframe::egui::{self, RichText, vec2};
use icons::Icon;
use serde_json::Value;
use std::time::Duration;

/// Right-anchored Running Devices tool window.
pub fn right(app: &mut IdeApp, ctx: &egui::Context) {
    if app.state.tool_windows().visible(Anchor::Right) != Some(ToolWindow::RunningDevices) {
        return;
    }
    egui::SidePanel::right("right_tool_window")
        .default_width(340.0)
        .min_width(240.0)
        .resizable(true)
        .show_separator_line(false)
        .frame(widgets::island_frame(ctx))
        .show(ctx, |ui| {
            let mirroring = app.state.android().mirror.is_running();
            let mut toggle = false;
            let hide = widgets::tool_window_header(ui, ToolWindow::RunningDevices.title(), |ui| {
                let (icon, tip) = if mirroring {
                    (Icon::Stop, "Stop mirroring")
                } else {
                    (Icon::Refresh, "Start mirroring")
                };
                toggle = icons::button(ui, icon, tip).clicked();
            });
            if hide {
                app.toggle_tool_window(ToolWindow::RunningDevices);
            }
            if toggle {
                let command = if mirroring {
                    "android.mirror.stop"
                } else {
                    "android.mirror.start"
                };
                app.dispatch(command, Value::Null);
            }
            if app.state.android().mirror.frame.is_some() && mirroring {
                mirror(app, ui);
                ctx.request_repaint_after(Duration::from_millis(100));
            } else {
                empty_state(app, ui);
            }
        });
}

fn empty_state(app: &mut IdeApp, ui: &mut egui::Ui) {
    let mut open_device_manager = false;
    ui.add_space(40.0);
    ui.vertical_centered(|ui| {
        icons::show(ui, Icon::Device, 48.0, *theme::TEXT_DISABLED);
        ui.add_space(12.0);
        ui.label(RichText::new("No devices are mirrored").color(theme::TEXT_BRIGHT));
        ui.add_space(6.0);
        ui.add(
            egui::Label::new(
                RichText::new(
                    "Connect a device over USB or Wi-Fi, or start an emulator, to see and \
                     control its screen here.",
                )
                .color(theme::TEXT_INFO),
            )
            .wrap(),
        );
        ui.add_space(8.0);
        if let Some(error) = &app.state.android().mirror.error {
            ui.label(RichText::new(error).color(theme::TEXT_INFO));
            ui.add_space(8.0);
        }
        if ui.link("Device Manager").clicked() {
            open_device_manager = true;
        }
    });
    if open_device_manager {
        app.dispatch("toolwindow.activate.deviceManager", Value::Null);
    }
}

fn mirror(app: &mut IdeApp, ui: &mut egui::Ui) {
    let mirror = &app.state.android().mirror;
    let Some(png) = mirror.frame.clone() else {
        return;
    };
    let frame_id = mirror.frame_id;
    let serial = mirror.serial.clone().unwrap_or_default();
    ui.label(RichText::new(serial).color(theme::TEXT_INFO));
    let key = egui::Id::new("running_devices_texture");
    let cached: Option<(u64, egui::TextureHandle)> = ui.data_mut(|d| d.get_temp(key));
    let texture = match cached {
        Some((id, texture)) if id == frame_id => texture,
        _ => {
            let Ok(decoded) = image::load_from_memory(&png) else {
                return;
            };
            let rgba = decoded.thumbnail(720, 1280).to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let texture =
                ui.ctx()
                    .load_texture("running_devices", color, egui::TextureOptions::LINEAR);
            ui.data_mut(|d| d.insert_temp(key, (frame_id, texture.clone())));
            texture
        }
    };
    let available = ui.available_size();
    let size = texture.size_vec2();
    let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
    ui.vertical_centered(|ui| {
        ui.image((texture.id(), vec2(size.x * scale, size.y * scale)));
    });
}
