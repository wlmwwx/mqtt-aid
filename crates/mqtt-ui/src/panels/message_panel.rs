//! The centre panel: the message monitor.

use egui::{RichText, Ui};
use mqtt_core::models::MqttMessage;

use crate::state::{ConnectionView, ExportFormat, UiActions, UiState};
use crate::widgets::icons::{self, Icon};
use crate::widgets::message_table;

/// The toolbar of the message monitor and the message table.
pub fn message_panel(
    ui: &mut Ui,
    state: &mut UiState,
    messages: &[MqttMessage],
    connections: &[ConnectionView],
    actions: &mut UiActions,
) {
    ui.horizontal(|ui| {
        icons::draw(
            ui.painter(),
            Icon::Search,
            egui::Rect::from_center_size(
                ui.cursor().left_center() + egui::vec2(7.0, 0.0),
                egui::vec2(14.0, 14.0),
            ),
            ui.visuals().weak_text_color(),
            1.3,
        );
        ui.add_space(14.0);

        let search_response = ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .hint_text("搜索 topic 或 payload…")
                .desired_width(180.0),
        );

        // The `查找` command asks for the keyboard focus here.
        if state.focus_search {
            state.focus_search = false;
            let id = search_response.id;
            ui.ctx().memory_mut(|memory| memory.request_focus(id));
        }

        egui::ComboBox::from_id_salt("filter-qos")
            .selected_text(match state.filter_qos {
                Some(qos) => format!("QoS {qos}"),
                None => "任意 QoS".to_string(),
            })
            .width(90.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(state.filter_qos.is_none(), "任意 QoS")
                    .clicked()
                {
                    state.filter_qos = None;
                }
                for qos in 0..=2u8 {
                    if ui
                        .selectable_label(state.filter_qos == Some(qos), format!("QoS {qos}"))
                        .clicked()
                    {
                        state.filter_qos = Some(qos);
                    }
                }
            });

        egui::ComboBox::from_id_salt("filter-retain")
            .selected_text(match state.filter_retain {
                Some(true) => "Retained".to_string(),
                Some(false) => "Live".to_string(),
                None => "全部".to_string(),
            })
            .width(90.0)
            .show_ui(ui, |ui| {
                for (value, label) in [
                    (None, "全部"),
                    (Some(true), "Retained"),
                    (Some(false), "Live"),
                ] {
                    if ui
                        .selectable_label(state.filter_retain == value, label)
                        .clicked()
                    {
                        state.filter_retain = value;
                    }
                }
            });

        egui::ComboBox::from_id_salt("filter-connection")
            .selected_text(
                state
                    .filter_connection
                    .as_ref()
                    .and_then(|id| connections.iter().find(|view| &view.config.id == id))
                    .map(|view| view.config.name.clone())
                    .unwrap_or_else(|| "全部连接".to_string()),
            )
            .width(120.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(state.filter_connection.is_none(), "全部连接")
                    .clicked()
                {
                    state.filter_connection = None;
                }
                for view in connections {
                    if ui
                        .selectable_label(
                            state.filter_connection.as_ref() == Some(&view.config.id),
                            view.config.name.clone(),
                        )
                        .clicked()
                    {
                        state.filter_connection = Some(view.config.id.clone());
                    }
                }
            });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_salt("export-format")
                .selected_text("导出…")
                .show_ui(ui, |ui| {
                    for (format, label) in [
                        (ExportFormat::Json, "导出为 JSON"),
                        (ExportFormat::Csv, "导出为 CSV"),
                        (ExportFormat::Hex, "导出为 HEX"),
                        (ExportFormat::Binary, "导出为原始字节"),
                    ] {
                        if ui.button(label).clicked() {
                            actions.export = Some(format);
                        }
                    }
                });

            let pause = if state.paused {
                Icon::Play
            } else {
                Icon::Pause
            };
            let tooltip = if state.paused {
                "继续接收"
            } else {
                "暂停接收"
            };
            if icons::icon_button(ui, pause, tooltip) {
                actions.toggle_pause = true;
            }

            if icons::icon_button(ui, Icon::Trash, "清除消息列表") {
                actions.clear_messages = true;
            }
        });
    });

    ui.separator();

    if messages.is_empty() {
        // An empty state that says what to do, not only that there is
        // nothing here.
        ui.centered_and_justified(|ui| {
            ui.add_space(48.0);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
            icons::draw(
                ui.painter(),
                Icon::Layers,
                rect,
                ui.visuals().weak_text_color().gamma_multiply(0.6),
                1.4,
            );
            ui.add_space(10.0);
            ui.label(RichText::new("还没有捕获到消息").color(ui.visuals().weak_text_color()));
            ui.label(
                RichText::new("订阅一个主题，消息就会出现在这里")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });
        return;
    }

    message_table(ui, messages, state, actions);
}
