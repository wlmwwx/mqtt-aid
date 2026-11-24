//! Modal dialogs: the connection editor, the subscribe form and the settings.

pub mod info;
pub mod series;
pub mod settings;

use egui::RichText;
use mqtt_core::models::{MqttVersion, PayloadFormat};

use crate::i18n;
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
        i18n::t("新建连接")
    } else {
        i18n::tc(Some("dialog"), "编辑连接")
    })
    .collapsible(false)
    .resizable(false)
    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
    .show(ctx, |ui| {
        ui.set_width(420.0);

        ui.label(i18n::t("名称"));
        ui.add(egui::TextEdit::singleline(&mut draft.config.name).desired_width(f32::INFINITY));

        ui.label(i18n::t("主机"));
        ui.add(egui::TextEdit::singleline(&mut draft.config.host).desired_width(f32::INFINITY));

        ui.label(i18n::t("端口"));
        ui.add(egui::DragValue::new(&mut draft.config.port).range(1..=65535));

        ui.label(i18n::t("协议"));
        ui.horizontal(|ui| {
            ui.selectable_label(draft.config.protocol == MqttVersion::V311, "MQTT 3.1.1")
                .clicked()
                .then(|| draft.config.protocol = MqttVersion::V311);
            ui.selectable_label(draft.config.protocol == MqttVersion::V5, "MQTT 5.0")
                .clicked()
                .then(|| draft.config.protocol = MqttVersion::V5);
        });

        ui.label(i18n::t("客户端 ID"));
        ui.add(
            egui::TextEdit::singleline(&mut draft.config.client_id).desired_width(f32::INFINITY),
        );

        ui.label(i18n::t("用户名"));
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

        ui.label(i18n::t("密码（保存在系统钥匙串里）"));
        ui.add(
            egui::TextEdit::singleline(&mut draft.password)
                .password(true)
                .desired_width(f32::INFINITY),
        );

        ui.label(i18n::t("保活（秒）"));
        ui.add(egui::DragValue::new(&mut draft.config.keep_alive).range(1..=3600));

        ui.checkbox(
            &mut draft.config.clean_session,
            i18n::t("清除会话 / 干净启动"),
        );
        ui.checkbox(&mut draft.config.auto_reconnect, i18n::t("自动重连"));

        ui.separator();
        ui.checkbox(&mut draft.config.tls, i18n::t("使用 TLS"));
        if draft.config.tls {
            ui.label(i18n::t("CA 证书"));
            let mut ca = draft.config.ca_certificate.clone().unwrap_or_default();
            if ui
                .add(egui::TextEdit::singleline(&mut ca).desired_width(f32::INFINITY))
                .changed()
            {
                draft.config.ca_certificate = if ca.trim().is_empty() { None } else { Some(ca) };
            }
            ui.label(i18n::t("客户端证书"));
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
            ui.label(i18n::t("客户端密钥"));
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
            if ui.button(i18n::t("保存")).clicked() {
                save = true;
            }
            if ui.button(i18n::t("取消")).clicked() {
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

/// The width of the subscribe form and of the fields in it.
const SUBSCRIBE_WIDTH: f32 = 380.0;

/// The subscribe form. Returns the draft when the user subscribes.
pub fn subscribe_dialog(
    state: &mut UiState,
    ctx: &egui::Context,
    connections: &[crate::state::ConnectionView],
) -> Option<crate::state::SubscribeDraft> {
    let draft = state.subscribe_draft.as_mut()?;

    let mut close = false;
    let mut save = false;

    egui::Window::new(i18n::tc(Some("button"), "订阅"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(SUBSCRIBE_WIDTH);

            // A subscription belongs to a connection, so the connection is
            // part of the form.
            ui.label(i18n::tc(Some("label"), "连接"));
            egui::ComboBox::from_id_salt("subscribe-connection")
                .selected_text(connection_name(connections, &draft.connection_id))
                // An infinite width would make the window's rect NaN, and
                // egui would stop with a panic.
                .width(SUBSCRIBE_WIDTH)
                .show_ui(ui, |ui| {
                    for connection in connections {
                        ui.selectable_label(
                            draft.connection_id == connection.config.id,
                            connection.config.name.clone(),
                        )
                        .clicked()
                        .then(|| draft.connection_id = connection.config.id.clone());
                    }
                });

            ui.label(i18n::t("主题过滤器"));
            ui.add(
                egui::TextEdit::singleline(&mut draft.topic)
                    .hint_text("device/+/status")
                    .desired_width(f32::INFINITY),
            );
            ui.label(i18n::t("服务质量"));
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
                if ui.button(i18n::tc(Some("button"), "订阅")).clicked() {
                    save = true;
                }
                if ui.button(i18n::t("取消")).clicked() {
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

/// The name of a connection, or a placeholder when it is gone.
fn connection_name(connections: &[crate::state::ConnectionView], connection_id: &str) -> String {
    connections
        .iter()
        .find(|connection| connection.config.id == connection_id)
        .map(|connection| connection.config.name.clone())
        .unwrap_or_else(|| i18n::t("（连接已删除）").to_string())
}
