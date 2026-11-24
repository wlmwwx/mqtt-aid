//! The right panel: publishing messages.

use egui::{RichText, Ui};
use mqtt_core::format::{FormatError, codec_for};
use mqtt_core::models::{PayloadFormat, PublishRequest};

use crate::i18n;
use crate::state::{ConnectionView, UiActions, UiState};
use crate::widgets::icons::{self, Icon};
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
    let action = panel_header::panel_header(ui, i18n::t("发布"), |ui| {
        if ui.button(i18n::t("清空表单")).clicked() {
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
        .unwrap_or_else(|| i18n::t("没有连接").to_string());

    ui.label(i18n::tc(Some("label"), "连接"));
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
    ui.label(i18n::t("主题"));
    ui.add(
        egui::TextEdit::singleline(&mut state.publish.topic)
            .hint_text("device/001/cmd")
            .desired_width(f32::INFINITY),
    );

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(i18n::t("格式"));
        egui::ComboBox::from_id_salt("publish-format")
            .selected_text(crate::state::format_label(state.publish.format))
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
                        .selectable_label(
                            state.publish.format == format,
                            crate::state::format_label(format),
                        )
                        .clicked()
                    {
                        state.publish.format = format;
                        state.publish.validation_error = None;
                    }
                }
            });
    });

    ui.add_space(4.0);
    ui.label(i18n::t("负载"));
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
        ui.label(i18n::t("缩进"));
        ui.add(
            egui::DragValue::new(&mut state.publish.json_indent)
                .range(0..=8)
                .speed(0.1),
        );
    });
    ui.horizontal(|ui| {
        if ui.button(i18n::t("格式化 JSON")).clicked() {
            actions.commands.push(crate::state::AppCommand::FormatJson);
        }
        if ui.button(i18n::t("压缩 JSON")).clicked() {
            actions.commands.push(crate::state::AppCommand::MinifyJson);
        }
    });

    let encoded = encode_payload(state.publish.format, &state.publish.payload);
    // Whether the text is an error is remembered, not read back from the
    // text: the text is translated, and the translation does not end with
    // "bytes".
    let failed = encoded.is_err();
    state.publish.validation_error = match &encoded {
        Ok(bytes) => Some(i18n::fill(i18n::t("{} 字节"), &[&bytes.len().to_string()])),
        Err(error) => Some(error.to_string()),
    };

    if let Some(message) = &state.publish.validation_error {
        ui.label(RichText::new(message.clone()).small().color(if failed {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().weak_text_color()
        }));
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(i18n::t("服务质量"));
        for qos in 0..=2u8 {
            let selected = state.publish.qos == qos;
            if ui.selectable_label(selected, qos.to_string()).clicked() {
                state.publish.qos = qos;
            }
        }
        ui.checkbox(&mut state.publish.retain, i18n::t("保留"));
    });

    let can_publish = state.publish.connection_id.is_some()
        && !state.publish.topic.trim().is_empty()
        && encoded.is_ok();

    // The publish button is the point of the panel: it gets the accent
    // colour and the whole width of the panel.
    let accent = ui.visuals().selection.stroke.color;
    let publish =
        ui.add_enabled(
            can_publish,
            egui::Button::new(egui::RichText::new(i18n::t("发布")).strong().color(
                if can_publish {
                    egui::Color32::WHITE
                } else {
                    ui.visuals().weak_text_color()
                },
            ))
            .fill(if can_publish {
                accent
            } else {
                // A muted fill, so that the button still reads as a button.
                ui.visuals().widgets.hovered.bg_fill
            })
            .stroke(egui::Stroke::NONE)
            .min_size(egui::vec2(ui.available_width(), 28.0)),
        );
    if publish.clicked()
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
    ui.label(RichText::new(i18n::t("模板")).strong());

    if !state.publish.templates.is_empty() {
        let templates = state.publish.templates.clone();
        for template in templates {
            ui.horizontal(|ui| {
                if ui.button(template.name.clone()).clicked() {
                    actions.load_template.push(template.id.clone());
                }
                if icons::icon_button(ui, Icon::Trash, i18n::t("删除模板")) {
                    actions.delete_template.push(template.id.clone());
                }
            });
        }
    } else {
        ui.label(
            RichText::new(i18n::t("还没有保存模板"))
                .small()
                .color(ui.visuals().weak_text_color())
                .italics(),
        );
    }

    if ui.button(i18n::t("保存当前为模板")).clicked()
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
