//! Reusable widgets for the message monitor.

pub mod icons;
pub mod panel_header;
pub mod payload_views;

use egui::{Color32, RichText, Ui};
use mqtt_core::models::{MqttMessage, PayloadFormat};

use crate::state::{CopyKind, CopyRequest, UiActions, UiState};
use crate::theme::spacing;

pub use payload_views::{hex_view, json_text_view, json_view, mono_text, monospace, preview};

/// Format an epoch millisecond stamp as `HH:MM:SS` (UTC).
pub fn format_time(millis: i64) -> String {
    let seconds = millis.div_euclid(1000);
    let day = seconds.div_euclid(86_400);
    let time_of_day = seconds.rem_euclid(86_400);
    let (hour, minute, second) = (
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60,
    );
    format!("{day:>3} {hour:02}:{minute:02}:{second:02}")
}

/// The table of received messages.
pub fn message_table(
    ui: &mut Ui,
    messages: &[MqttMessage],
    state: &mut UiState,
    actions: &mut UiActions,
) {
    let row_height = spacing::ROW_HEIGHT;
    let text_dim = ui.visuals().weak_text_color();

    // The table gets its own scroll area, and the detail view below it the
    // rest of the panel. Both need their own id, or egui complains about a
    // doubled id.
    let table_height = (ui.available_height() - spacing::DETAILS_HEIGHT).max(60.0);
    egui::ScrollArea::vertical()
        .id_salt("message_table")
        .max_height(table_height)
        .show_rows(ui, row_height, messages.len(), |ui, range| {
            for index in range {
                let message = &messages[index];
                let selected = state.selected_message == Some(index);
                let mut clicked = false;

                let response = ui.horizontal(|ui| {
                    let time = ui
                        .add(
                            egui::Label::new(
                                RichText::new(format_time(message.received_at)).color(text_dim),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_text("Received at");
                    let topic = ui
                        .add(
                            egui::Label::new(RichText::new(message.topic.clone()).strong())
                                .sense(egui::Sense::click()),
                        )
                        .on_hover_text("Topic");
                    let qos = ui
                        .add(
                            egui::Label::new(
                                RichText::new(format!("{}", message.qos)).color(text_dim),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_text("QoS");
                    let retain = ui
                        .add(
                            egui::Label::new(
                                RichText::new(if message.retain { "R" } else { "" })
                                    .color(Color32::from_rgb(214, 168, 92)),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_text("Retained");
                    let size = ui
                        .add(
                            egui::Label::new(
                                RichText::new(format!("{} B", message.payload.len()))
                                    .color(text_dim),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_text("Payload size");
                    ui.add(
                        egui::Label::new(
                            RichText::new(preview(&message.payload, 64)).color(text_dim),
                        )
                        .sense(egui::Sense::click()),
                    );

                    clicked = time.clicked()
                        || topic.clicked()
                        || qos.clicked()
                        || retain.clicked()
                        || size.clicked();

                    // The right mouse button opens the context menu of the row.
                    time.context_menu(|ui| {
                        crate::panels::menu_bar::message_context_menu(ui, actions);
                    });
                });

                if clicked {
                    state.selected_message = Some(index);
                    // The copy commands of the menu bar act on this message.
                    state.copy_target = Some(message.clone());
                }

                if selected {
                    ui.painter().rect_filled(
                        response.response.rect,
                        spacing::CORNER,
                        ui.visuals().selection.bg_fill,
                    );
                }
            }
        });

    if state.selected_message.is_some() {
        let index = state.selected_message.unwrap();
        if let Some(message) = messages.get(index) {
            ui.separator();
            message_details(ui, message, state, actions);
        }
    }
}

/// The detail view of the selected message.
fn message_details(
    ui: &mut Ui,
    message: &MqttMessage,
    state: &mut UiState,
    actions: &mut UiActions,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(message.topic.clone()).strong());
        ui.label(
            RichText::new(format!(
                "{} · QoS {} · {} · {} bytes · MQTT {}",
                format_time(message.received_at),
                message.qos,
                if message.retain { "retained" } else { "live" },
                message.payload.len(),
                message.version.label()
            ))
            .color(ui.visuals().weak_text_color()),
        );
    });

    ui.horizontal(|ui| {
        if ui.button("Copy topic").clicked() {
            actions.copy.push(CopyRequest {
                kind: CopyKind::Topic,
                value: message.topic.clone(),
            });
        }
        if ui.button("Copy payload").clicked() {
            actions.copy.push(CopyRequest {
                kind: CopyKind::Payload,
                value: String::from_utf8_lossy(&message.payload).to_string(),
            });
        }
        if ui.button("Copy hex").clicked() {
            actions.copy.push(CopyRequest {
                kind: CopyKind::Hex,
                value: mqtt_core::format::encode_hex(&message.payload),
            });
        }
        if ui.button("Send to publisher").clicked() {
            actions.send_to_publisher = Some(message.clone());
        }
    });

    if let Some(properties) = &message.properties {
        egui::CollapsingHeader::new("MQTT 5.0 properties")
            .default_open(false)
            .show(ui, |ui| {
                if let Some(content_type) = &properties.content_type {
                    ui.label(format!("Content type: {content_type}"));
                }
                if let Some(indicator) = properties.payload_format_indicator {
                    ui.label(format!("Payload format indicator: {indicator}"));
                }
                if let Some(expiry) = properties.message_expiry_interval {
                    ui.label(format!("Message expiry interval: {expiry} s"));
                }
                if let Some(response_topic) = &properties.response_topic {
                    ui.label(format!("Response topic: {response_topic}"));
                }
                if let Some(correlation) = &properties.correlation_data {
                    ui.label(format!(
                        "Correlation data: {}",
                        mqtt_core::format::encode_hex(correlation)
                    ));
                }
                for (key, value) in &properties.user_properties {
                    ui.label(format!("User property {key} = {value}"));
                }
            });
    }

    ui.horizontal(|ui| {
        ui.label("Format:");
        for format in [
            PayloadFormat::Auto,
            PayloadFormat::Text,
            PayloadFormat::Hex,
            PayloadFormat::Json,
            PayloadFormat::Base64,
            PayloadFormat::Binary,
        ] {
            let selected = state.message_format == format;
            if ui.selectable_label(selected, format.label()).clicked() {
                state.message_format = format;
            }
        }
    });

    let resolved = state.message_format.resolve(&message.payload);
    match resolved {
        PayloadFormat::Hex => hex_view(ui, &message.payload),
        PayloadFormat::Json => {
            let text = String::from_utf8_lossy(&message.payload).to_string();
            json_text_view(ui, &text);
        }
        PayloadFormat::Base64 => {
            let mut text = mqtt_core::format::codec_for(PayloadFormat::Base64)
                .decode(&message.payload)
                .unwrap_or_default();
            monospace(ui, &mut text);
        }
        PayloadFormat::Binary => {
            let mut text = mqtt_core::format::codec_for(PayloadFormat::Binary)
                .decode(&message.payload)
                .unwrap_or_default();
            monospace(ui, &mut text);
        }
        _ => {
            let mut text = String::from_utf8_lossy(&message.payload).to_string();
            monospace(ui, &mut text);
        }
    }
}
