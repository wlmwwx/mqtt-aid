//! The right panel: publishing messages.

use egui::{RichText, Ui};
use mqtt_core::format::{FormatError, codec_for};
use mqtt_core::models::{PayloadFormat, PublishRequest};

use crate::state::{ConnectionView, UiActions, UiState};
use crate::widgets::panel_header::{self, HeaderAction};

/// Encode the payload according to the selected format.
pub fn encode_payload(format: PayloadFormat, text: &str) -> Result<Vec<u8>, FormatError> {
    match format {
        PayloadFormat::Hex => codec_for(PayloadFormat::Hex).encode(text),
        PayloadFormat::Json => codec_for(PayloadFormat::Json).encode(text),
        PayloadFormat::Base64 => codec_for(PayloadFormat::Base64).encode(text),
        PayloadFormat::Binary => codec_for(PayloadFormat::Binary).encode(text),
        _ => Ok(text.as_bytes().to_vec()),
    }
}

/// The publish form and the template list.
pub fn publish_panel(
    ui: &mut Ui,
    state: &mut UiState,
    connections: &[ConnectionView],
    actions: &mut UiActions,
) {
    let action = panel_header::panel_header(ui, "发布", |ui| {
        if ui.button("清空表单").clicked() {
            state.publish.topic.clear();
            state.publish.payload.clear();
        }
    });
    if action == HeaderAction::Collapse {
        state.right_collapsed = !state.right_collapsed;
    }

    if state.right_collapsed {
        return;
    }

    ui.add_space(4.0);

    let selected_name = state
        .publish
        .connection_id
        .as_ref()
        .and_then(|id| connections.iter().find(|view| &view.config.id == id))
        .map(|view| view.config.name.clone())
        .unwrap_or_else(|| "No connection".to_string());

    let combo_width = ui.available_width();
    egui::ComboBox::from_id_salt("publish-connection")
        .selected_text(selected_name)
        .width(combo_width)
        .show_ui(ui, |ui| {
            for view in connections {
                let selected = state.publish.connection_id.as_ref() == Some(&view.config.id);
                if ui
                    .selectable_label(selected, view.config.name.clone())
                    .clicked()
                {
                    state.publish.connection_id = Some(view.config.id.clone());
                }
            }
        });

    ui.add_space(4.0);
    ui.label("Topic（主题）");
    ui.add(
        egui::TextEdit::singleline(&mut state.publish.topic)
            .hint_text("device/001/cmd")
            .desired_width(f32::INFINITY),
    );

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("格式");
        egui::ComboBox::from_id_salt("publish-format")
            .selected_text(state.publish.format.label())
            .width(110.0)
            .show_ui(ui, |ui| {
                for format in [
                    PayloadFormat::Text,
                    PayloadFormat::Hex,
                    PayloadFormat::Json,
                    PayloadFormat::Base64,
                    PayloadFormat::Binary,
                ] {
                    if ui
                        .selectable_label(state.publish.format == format, format.label())
                        .clicked()
                    {
                        state.publish.format = format;
                        state.publish.validation_error = None;
                    }
                }
            });
    });

    ui.add_space(4.0);
    ui.label("Payload（负载）");
    ui.add(
        egui::TextEdit::multiline(&mut state.publish.payload)
            .desired_width(f32::INFINITY)
            .desired_rows(8)
            .hint_text(match state.publish.format {
                PayloadFormat::Hex => "01 03 00 00 00 0A C5 CD",
                PayloadFormat::Json => "{ \"cmd\": \"start\" }",
                _ => "payload",
            }),
    );

    // The JSON tools, next to the payload they work on.
    ui.horizontal(|ui| {
        ui.label("缩进");
        ui.add(
            egui::DragValue::new(&mut state.publish.json_indent)
                .range(0..=8)
                .speed(0.1),
        );
    });
    ui.horizontal(|ui| {
        if ui.button("格式化 JSON").clicked() {
            actions.commands.push(crate::state::AppCommand::FormatJson);
        }
        if ui.button("压缩 JSON").clicked() {
            actions.commands.push(crate::state::AppCommand::MinifyJson);
        }
    });

    let encoded = encode_payload(state.publish.format, &state.publish.payload);
    state.publish.validation_error = match &encoded {
        Ok(bytes) => Some(format!("{} bytes", bytes.len())),
        Err(error) => Some(error.to_string()),
    };

    if let Some(message) = &state.publish.validation_error {
        let is_error = !message.ends_with("bytes");
        ui.label(RichText::new(message.clone()).small().color(if is_error {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().weak_text_color()
        }));
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("QoS（服务质量）");
        for qos in 0..=2u8 {
            let selected = state.publish.qos == qos;
            if ui.selectable_label(selected, qos.to_string()).clicked() {
                state.publish.qos = qos;
            }
        }
        ui.checkbox(&mut state.publish.retain, "Retain（保留）");
    });

    let can_publish = state.publish.connection_id.is_some()
        && !state.publish.topic.trim().is_empty()
        && encoded.is_ok();

    if ui
        .add_enabled(can_publish, egui::Button::new("发布"))
        .clicked()
        && let (Some(connection_id), Ok(payload)) =
            (state.publish.connection_id.clone(), encoded.clone())
    {
        actions.publish.push(PublishRequest {
            connection_id,
            topic: state.publish.topic.trim().to_string(),
            payload,
            qos: state.publish.qos,
            retain: state.publish.retain,
            properties: None,
        });
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("模板").strong());

    if !state.publish.templates.is_empty() {
        let templates = state.publish.templates.clone();
        for template in templates {
            ui.horizontal(|ui| {
                if ui.button(template.name.clone()).clicked() {
                    actions.load_template.push(template.id.clone());
                }
                if ui.button("🗑").on_hover_text("删除模板").clicked() {
                    actions.delete_template.push(template.id.clone());
                }
            });
        }
    } else {
        ui.label(
            RichText::new("还没有保存模板")
                .small()
                .color(ui.visuals().weak_text_color())
                .italics(),
        );
    }

    if ui.button("保存当前为模板").clicked()
        && let (Some(connection_id), Ok(payload)) =
            (state.publish.connection_id.clone(), encoded.clone())
    {
        actions
            .save_template
            .push(mqtt_core::models::PublishTemplate {
                id: crate::panels::left_panel::new_id(),
                name: format!("{} @ {}", state.publish.topic, connection_id),
                topic: state.publish.topic.trim().to_string(),
                payload,
                format: state.publish.format,
                qos: state.publish.qos,
                retain: state.publish.retain,
            });
    }
}
