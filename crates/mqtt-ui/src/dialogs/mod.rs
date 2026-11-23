//! Modal dialogs: the connection editor, the subscribe form and the settings.

pub mod info;
pub mod series;
pub mod settings;

use egui::RichText;
use mqtt_core::models::{MqttVersion, PayloadFormat};

use crate::state::UiState;

/// The connection editor. Returns the draft when the user saves it.
pub fn connection_dialog(
    ctx: &egui::Context,
    state: &mut UiState,
) -> Option<crate::state::ConnectionDraft> {
    let draft = state.connection_draft.as_mut()?;

    let mut close = false;
    let mut save = false;

    egui::Window::new(if draft.is_new {
        "New connection"
    } else {
        "Edit connection"
    })
    .collapsible(false)
    .resizable(false)
    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
    .show(ctx, |ui| {
        ui.set_width(420.0);

        ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut draft.config.name).desired_width(f32::INFINITY));

        ui.label("Host");
        ui.add(egui::TextEdit::singleline(&mut draft.config.host).desired_width(f32::INFINITY));

        ui.label("Port");
        ui.add(egui::DragValue::new(&mut draft.config.port).range(1..=65535));

        ui.label("Protocol");
        ui.horizontal(|ui| {
            ui.selectable_label(draft.config.protocol == MqttVersion::V311, "MQTT 3.1.1")
                .clicked()
                .then(|| draft.config.protocol = MqttVersion::V311);
            ui.selectable_label(draft.config.protocol == MqttVersion::V5, "MQTT 5.0")
                .clicked()
                .then(|| draft.config.protocol = MqttVersion::V5);
        });

        ui.label("Client ID");
        ui.add(
            egui::TextEdit::singleline(&mut draft.config.client_id).desired_width(f32::INFINITY),
        );

        ui.label("Username");
        let mut username = draft.config.username.clone().unwrap_or_default();
        if ui
            .add(egui::TextEdit::singleline(&mut username).desired_width(f32::INFINITY))
            .changed()
        {
            draft.config.username = if username.trim().is_empty() {
                None
            } else {
                Some(username)
            };
        }

        ui.label("Password (stored in the system keyring)");
        ui.add(
            egui::TextEdit::singleline(&mut draft.password)
                .password(true)
                .desired_width(f32::INFINITY),
        );

        ui.label("Keep alive (seconds)");
        ui.add(egui::DragValue::new(&mut draft.config.keep_alive).range(1..=3600));

        ui.checkbox(
            &mut draft.config.clean_session,
            "Clean session / clean start",
        );
        ui.checkbox(&mut draft.config.auto_reconnect, "Automatic reconnect");

        ui.separator();
        ui.checkbox(&mut draft.config.tls, "Use TLS");
        if draft.config.tls {
            ui.label("CA certificate");
            let mut ca = draft.config.ca_certificate.clone().unwrap_or_default();
            if ui
                .add(egui::TextEdit::singleline(&mut ca).desired_width(f32::INFINITY))
                .changed()
            {
                draft.config.ca_certificate = if ca.trim().is_empty() { None } else { Some(ca) };
            }
            ui.label("Client certificate");
            let mut cert = draft.config.client_certificate.clone().unwrap_or_default();
            if ui
                .add(egui::TextEdit::singleline(&mut cert).desired_width(f32::INFINITY))
                .changed()
            {
                draft.config.client_certificate = if cert.trim().is_empty() {
                    None
                } else {
                    Some(cert)
                };
            }
            ui.label("Client key");
            let mut key = draft.config.client_key.clone().unwrap_or_default();
            if ui
                .add(egui::TextEdit::singleline(&mut key).desired_width(f32::INFINITY))
                .changed()
            {
                draft.config.client_key = if key.trim().is_empty() {
                    None
                } else {
                    Some(key)
                };
            }
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                save = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });

    if save {
        state.connection_draft.take()
    } else {
        if close {
            state.connection_draft = None;
        }
        None
    }
}

/// The subscribe form. Returns the draft when the user subscribes.
pub fn subscribe_dialog(
    state: &mut UiState,
    ctx: &egui::Context,
) -> Option<crate::state::SubscribeDraft> {
    let draft = state.subscribe_draft.as_mut()?;

    let mut close = false;
    let mut save = false;

    egui::Window::new("Subscribe")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(380.0);
            ui.label("Topic filter");
            ui.add(
                egui::TextEdit::singleline(&mut draft.topic)
                    .hint_text("device/+/status")
                    .desired_width(f32::INFINITY),
            );
            ui.label("QoS");
            ui.horizontal(|ui| {
                for qos in 0..=2u8 {
                    let selected = draft.qos == qos;
                    if ui.selectable_label(selected, qos.to_string()).clicked() {
                        draft.qos = qos;
                    }
                }
            });
            ui.label(
                RichText::new("`+` matches one level, `#` matches the rest of the tree")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.horizontal(|ui| {
                if ui.button("Subscribe").clicked() {
                    save = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });

    if save {
        state.subscribe_draft.take()
    } else {
        if close {
            state.subscribe_draft = None;
        }
        None
    }
}

/// The default format for a new subscription.
pub fn default_format() -> PayloadFormat {
    PayloadFormat::Auto
}
