//! Android device selector (main toolbar) and the Device Manager tool window.

use crate::{IdeApp, icons, theme, widgets};
use archipel_ide_android::api_release_name;
use archipel_ide_core::{Anchor, ToolWindow};
use eframe::egui::{self, Color32, Rect, RichText, Sense, vec2};
use icons::Icon;
use serde_json::{Value, json};

/// Label of the toolbar device selector.
pub fn selector_label(app: &IdeApp) -> (String, bool) {
    let hub = &app.state.android.devices;
    match hub.selected_device() {
        Some(device) => (device.display_name(), true),
        None if hub.devices.is_empty() => ("No Devices".into(), false),
        None => ("No Device Selected".into(), false),
    }
}

/// Device selector shown left of the run configuration for Android projects.
pub fn device_selector(app: &mut IdeApp, ui: &mut egui::Ui) {
    if app.state.android.project.is_none() {
        return;
    }
    let (label, active) = selector_label(app);
    let icon_color = if active {
        icons::ICON
    } else {
        theme::TEXT_DISABLED
    };
    let response = widgets::flat_widget(
        ui,
        Some((Icon::Device, *icon_color)),
        &label,
        egui::FontId::proportional(theme::UI_FONT_SIZE),
        if active {
            *theme::TEXT_BRIGHT
        } else {
            *theme::TEXT_INFO
        },
        true,
        28.0,
    );
    egui::Popup::menu(&response).show(|ui| {
        ui.set_min_width(260.0);
        let hub = &app.state.android.devices;
        let mut select = None;
        let mut start = None;
        ui.label(RichText::new("Running devices").color(theme::TEXT_INFO));
        if hub.devices.is_empty() {
            ui.label(RichText::new("  None").color(theme::TEXT_DISABLED));
        }
        for device in &hub.devices {
            let selected = hub.selected.as_deref() == Some(device.serial.as_str());
            let mut text = device.display_name();
            if !device.is_online() {
                text.push_str(&format!(" [{}]", device.state.label()));
            }
            if ui.selectable_label(selected, text).clicked() {
                select = Some(device.serial.clone());
                ui.close();
            }
        }
        let available: Vec<_> = hub
            .avds
            .iter()
            .filter(|avd| hub.running_avd(&avd.name).is_none())
            .collect();
        if !available.is_empty() {
            ui.separator();
            ui.label(RichText::new("Available devices").color(theme::TEXT_INFO));
            for avd in available {
                let launching = hub.is_launching(&avd.name);
                let text = if launching {
                    format!("{} (starting…)", avd.display_name)
                } else {
                    avd.display_name.clone()
                };
                if ui
                    .add_enabled(!launching, egui::Button::new(text))
                    .on_hover_text("Start this virtual device")
                    .clicked()
                {
                    start = Some(avd.name.clone());
                    ui.close();
                }
            }
        }
        ui.separator();
        let can_run = app.commands.is_enabled("android.app.run", &app.state)
            && hub.selected_device().is_some_and(|d| d.is_online());
        let run_label = match hub.selected_device() {
            Some(device) => format!("Run 'app' on {}", device.display_name()),
            None => "Run 'app'".into(),
        };
        let mut run = false;
        if ui
            .add_enabled(can_run, egui::Button::new(run_label))
            .clicked()
        {
            run = true;
            ui.close();
        }
        if ui.button("Device Manager").clicked() {
            app.dispatch("toolwindow.activate.deviceManager", Value::Null);
            ui.close();
        }
        if let Some(serial) = select {
            app.dispatch("android.devices.select", json!({"serial": serial}));
        }
        if let Some(avd) = start {
            app.dispatch("android.emulator.start", json!({"avd": avd}));
        }
        if run {
            app.dispatch("android.app.run", Value::Null);
        }
    });
}

fn api_text(api: Option<&str>) -> String {
    api.unwrap_or("—").to_owned()
}

fn release_text(api: Option<&str>) -> Option<String> {
    api.and_then(|a| a.split(['.', '-']).next()?.parse().ok())
        .and_then(api_release_name)
}

struct Row {
    name: String,
    detail: String,
    api: String,
    kind: String,
    /// `Some(true)` running, `Some(false)` stopped, `None` physical device.
    running: Option<bool>,
    launching: bool,
    avd: Option<String>,
    status: Option<(String, Color32)>,
}

fn rows(app: &IdeApp) -> Vec<Row> {
    let hub = &app.state.android.devices;
    let mut rows: Vec<Row> = hub
        .avds
        .iter()
        .map(|avd| {
            let api = avd.api_level.as_deref();
            let mut detail = release_text(api).unwrap_or_else(|| "Unknown API".into());
            if let Some(abi) = &avd.abi {
                detail.push_str(&format!(" | {abi}"));
            }
            let running = hub.running_avd(&avd.name).is_some();
            Row {
                name: avd.display_name.clone(),
                detail,
                api: api_text(api),
                kind: avd.kind.label().into(),
                running: Some(running),
                launching: hub.is_launching(&avd.name),
                avd: Some(avd.name.clone()),
                status: None,
            }
        })
        .collect();
    for device in hub.devices.iter().filter(|d| !d.is_emulator()) {
        let api = device.api_level();
        let status = (!device.is_online()).then(|| {
            let hint = match device.state {
                archipel_ide_android::DeviceState::Unauthorized => {
                    "Unauthorized: accept the debugging prompt on the device"
                }
                _ => device.state.label(),
            };
            (hint.to_owned(), *theme::WARNING)
        });
        rows.push(Row {
            name: device.display_name(),
            detail: format!(
                "{} | {}",
                release_text(api).unwrap_or_else(|| "Physical device".into()),
                device.serial
            ),
            api: api_text(api),
            kind: "Phone".into(),
            running: None,
            launching: false,
            avd: None,
            status,
        });
    }
    // Emulators not started from a known AVD home (e.g. another AVD folder).
    for device in hub.devices.iter().filter(|d| {
        d.is_emulator()
            && !d
                .properties
                .as_ref()
                .and_then(|p| p.avd_name.as_deref())
                .is_some_and(|name| hub.avds.iter().any(|a| a.name == name))
    }) {
        rows.push(Row {
            name: device.display_name(),
            detail: format!("Emulator | {}", device.serial),
            api: api_text(device.api_level()),
            kind: "Phone".into(),
            running: Some(device.is_online()),
            launching: false,
            avd: None,
            status: None,
        });
    }
    rows
}

/// Right-anchored Device Manager tool window.
pub fn right(app: &mut IdeApp, ctx: &egui::Context) {
    let Some(window) = app.state.tool_windows.visible(Anchor::Right) else {
        return;
    };
    if window != ToolWindow::DeviceManager {
        return;
    }
    egui::SidePanel::right("right_tool_window")
        .default_width(520.0)
        .min_width(280.0)
        .resizable(true)
        .show_separator_line(false)
        .frame(widgets::island_frame(ctx))
        .show(ctx, |ui| {
            let hide = widgets::tool_window_header(ui, window.title(), |ui| {
                if icons::button(ui, Icon::Refresh, "Refresh devices").clicked() {
                    app.dispatch("android.avd.list", Value::Null);
                    if !app.state.android.devices.is_tracking() {
                        app.dispatch("android.devices.track", Value::Null);
                    }
                }
            });
            if hide {
                app.toggle_tool_window(window);
            }
            let id = egui::Id::new("device_manager_state");
            let mut dm: DmState = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
            dm_toolbar(app, ui, &mut dm);
            device_table(app, ui, &mut dm);
            dm_dialogs(app, ui.ctx(), &mut dm);
            ui.data_mut(|d| d.insert_temp(id, dm));
        });
}

/// UI-only Device Manager state, kept in egui memory.
#[derive(Debug, Clone, Default)]
struct DmState {
    filter: String,
    dialog: Option<DmDialog>,
    name: String,
    image: String,
    profile: String,
    address: String,
    code: String,
    message: Option<(String, bool)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DmDialog {
    Create,
    Pair,
}

fn add_device_menu(ui: &mut egui::Ui, dm: &mut DmState) {
    ui.set_min_width(220.0);
    if ui.button("Create Virtual Device").clicked() {
        dm.dialog = Some(DmDialog::Create);
        dm.message = None;
        ui.close();
    }
    if ui.button("Pair Devices Using Wi-Fi").clicked() {
        dm.dialog = Some(DmDialog::Pair);
        dm.message = None;
        ui.close();
    }
}

fn dm_toolbar(app: &mut IdeApp, ui: &mut egui::Ui, dm: &mut DmState) {
    let _ = app;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let add = widgets::flat_widget(
            ui,
            Some((Icon::Plus, *icons::ICON)),
            "",
            egui::FontId::proportional(theme::UI_FONT_SIZE),
            *theme::TEXT,
            true,
            26.0,
        );
        egui::Popup::menu(&add).show(|ui| add_device_menu(ui, dm));
        add.on_hover_text("Add a new device");
        ui.add(
            egui::TextEdit::singleline(&mut dm.filter)
                .hint_text("Filter devices")
                .desired_width((ui.available_width() - 40.0).max(80.0)),
        );
        icons::button(ui, Icon::Search, "Filter devices by name, API or type");
    });
    ui.add_space(2.0);
}

fn dm_dialogs(app: &mut IdeApp, ctx: &egui::Context, dm: &mut DmState) {
    let Some(kind) = dm.dialog else { return };
    let mut close = false;
    let title = match kind {
        DmDialog::Create => "Create Virtual Device",
        DmDialog::Pair => "Pair Devices Using Wi-Fi",
    };
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .show(ctx, |ui| {
            match kind {
                DmDialog::Create => {
                    let images: Vec<String> = app
                        .state
                        .android
                        .sdk
                        .as_ref()
                        .and_then(|d| d.sdk.as_ref())
                        .map(|sdk| {
                            sdk.packages
                                .iter()
                                .filter(|p| p.category == archipel_ide_android::PackageCategory::SystemImage)
                                .map(|p| p.path.clone())
                                .collect()
                        })
                        .unwrap_or_default();
                    egui::Grid::new("dm_create").num_columns(2).show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut dm.name);
                        ui.end_row();
                        ui.label("System image");
                        ui.add(
                            egui::TextEdit::singleline(&mut dm.image)
                                .hint_text("system-images;android-35;google_apis;x86_64"),
                        );
                        ui.end_row();
                        ui.label("Device profile");
                        ui.add(egui::TextEdit::singleline(&mut dm.profile).hint_text("pixel_6 (optional)"));
                        ui.end_row();
                    });
                    if !images.is_empty() {
                        ui.label(RichText::new("Installed system images").color(theme::TEXT_INFO));
                        for image in images {
                            if ui.link(&image).clicked() {
                                dm.image = image;
                            }
                        }
                    }
                }
                DmDialog::Pair => {
                    ui.label(
                        RichText::new("On the device: Developer options › Wireless debugging › Pair device with pairing code.")
                            .color(theme::TEXT_INFO),
                    );
                    egui::Grid::new("dm_pair").num_columns(2).show(ui, |ui| {
                        ui.label("Address");
                        ui.add(egui::TextEdit::singleline(&mut dm.address).hint_text("192.168.1.20:37123"));
                        ui.end_row();
                        ui.label("Pairing code");
                        ui.add(egui::TextEdit::singleline(&mut dm.code).hint_text("123456"));
                        ui.end_row();
                    });
                }
            }
            if let Some((message, ok)) = &dm.message {
                ui.label(RichText::new(message).color(if *ok { *theme::SUCCESS } else { *theme::ERROR }));
            }
            ui.horizontal(|ui| {
                let label = if kind == DmDialog::Create { "Create" } else { "Pair" };
                if ui.button(label).clicked() {
                    let result = match kind {
                        DmDialog::Create => app.commands.execute(
                            "android.avd.create",
                            &mut app.state,
                            json!({"name": dm.name.trim(), "systemImage": dm.image.trim(), "device": dm.profile.trim()}),
                        ),
                        DmDialog::Pair => app.commands.execute(
                            "android.devices.pair",
                            &mut app.state,
                            json!({"address": dm.address.trim(), "code": dm.code.trim()}),
                        ),
                    };
                    match result {
                        Ok(_) => close = true,
                        Err(error) => dm.message = Some((error.to_string(), false)),
                    }
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
    if close {
        dm.dialog = None;
    }
}

fn device_table(app: &mut IdeApp, ui: &mut egui::Ui, dm: &mut DmState) {
    let width = ui.available_width();
    let api_x = (width - 190.0).max(140.0);
    let type_x = api_x + 52.0;
    let actions_x = width - 44.0;
    let (header, _) = ui.allocate_exact_size(vec2(width, 26.0), Sense::hover());
    let painter = ui.painter();
    let text = |painter: &egui::Painter, x: f32, y: f32, label: &str, font, color| {
        painter.text(
            egui::pos2(x, y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            color,
        );
    };
    let font = || egui::FontId::proportional(theme::UI_FONT_SIZE);
    text(
        painter,
        header.left() + 46.0,
        header.center().y,
        "Name",
        font(),
        *theme::TEXT,
    );
    text(
        painter,
        header.left() + api_x,
        header.center().y,
        "API",
        font(),
        *theme::TEXT,
    );
    text(
        painter,
        header.left() + type_x,
        header.center().y,
        "Type",
        font(),
        *theme::TEXT,
    );
    painter.hline(
        header.x_range(),
        header.bottom(),
        egui::Stroke::new(1.0_f32, theme::BORDER),
    );
    if let Some(error) = &app.state.android.devices.error {
        let error = error.split_whitespace().collect::<Vec<_>>().join(" ");
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                ui.label(RichText::new(error).color(theme::ERROR));
            });
    }
    let mut rows = rows(app);
    let needle = dm.filter.trim().to_lowercase();
    if !needle.is_empty() {
        rows.retain(|r| {
            [&r.name, &r.api, &r.kind, &r.detail]
                .iter()
                .any(|f| f.to_lowercase().contains(&needle))
        });
    }
    if rows.is_empty() {
        ui.add_space((ui.available_height() / 2.0 - 40.0).max(12.0));
        ui.vertical_centered(|ui| {
            let text = if needle.is_empty() {
                "No devices connected."
            } else {
                "No devices match the filter."
            };
            ui.label(RichText::new(text).color(theme::TEXT_INFO));
            ui.add_space(4.0);
            let link = ui.link(RichText::new("Add a new device...").color(theme::MODIFIED));
            egui::Popup::menu(&link).show(|ui| add_device_menu(ui, dm));
        });
        return;
    }
    let mut action: Option<(&str, String)> = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for row in &rows {
                let (rect, response) = ui.allocate_exact_size(vec2(width, 44.0), Sense::hover());
                let painter = ui.painter();
                if response.hovered() {
                    painter.rect_filled(rect.shrink2(vec2(4.0, 0.0)), 4.0, theme::HOVER);
                }
                let icon_color = match row.running {
                    Some(true) => theme::SUCCESS,
                    _ => icons::ICON,
                };
                icons::paint(
                    painter,
                    Rect::from_center_size(
                        egui::pos2(rect.left() + 26.0, rect.center().y),
                        vec2(16.0, 16.0),
                    ),
                    Icon::Device,
                    *icon_color,
                );
                text(
                    painter,
                    rect.left() + 46.0,
                    rect.top() + 14.0,
                    &row.name,
                    theme::semibold(theme::UI_FONT_SIZE),
                    *theme::TEXT_BRIGHT,
                );
                let (detail, detail_color) = row
                    .status
                    .clone()
                    .unwrap_or((row.detail.clone(), *theme::TEXT_INFO));
                text(
                    painter,
                    rect.left() + 46.0,
                    rect.top() + 31.0,
                    &detail,
                    egui::FontId::proportional(theme::UI_FONT_SIZE - 1.0),
                    detail_color,
                );
                text(
                    painter,
                    rect.left() + api_x,
                    rect.center().y,
                    &row.api,
                    font(),
                    *theme::TEXT,
                );
                text(
                    painter,
                    rect.left() + type_x,
                    rect.center().y,
                    &row.kind,
                    font(),
                    *theme::TEXT,
                );
                let Some(avd) = &row.avd else { continue };
                let button_rect = Rect::from_center_size(
                    egui::pos2(rect.left() + actions_x + 12.0, rect.center().y),
                    vec2(26.0, 26.0),
                );
                let button = ui.interact(
                    button_rect,
                    ui.id().with(("avd_action", avd)),
                    Sense::click(),
                );
                let (icon, color, tooltip) = match (row.running, row.launching) {
                    (Some(true), _) => (Icon::Stop, theme::STOP_RED, "Stop"),
                    (_, true) => (Icon::Run, theme::TEXT_DISABLED, "Starting…"),
                    _ => (Icon::Run, theme::RUN_GREEN, "Start"),
                };
                if button.hovered() {
                    ui.painter().rect_filled(button_rect, 6.0, theme::HOVER);
                }
                icons::paint(
                    ui.painter(),
                    Rect::from_center_size(button_rect.center(), vec2(14.0, 14.0)),
                    icon,
                    *color,
                );
                if button.on_hover_text(tooltip).clicked() && !row.launching {
                    let id = if row.running == Some(true) {
                        "android.emulator.stop"
                    } else {
                        "android.emulator.start"
                    };
                    action = Some((id, avd.clone()));
                }
            }
        });
    if let Some((id, avd)) = action {
        app.dispatch(id, json!({"avd": avd}));
    }
}
