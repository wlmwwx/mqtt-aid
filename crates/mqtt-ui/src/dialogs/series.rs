//! The editor of a data series.

use egui::RichText;
use mqtt_core::models::MqttSubscription;

use crate::ConnectionView;
use crate::state::{SeriesDraft, UiState};

/// The width of the dialog and of the fields in it.
const WIDTH: f32 = 400.0;

/// The series editor. Returns the draft when the user saves it.
pub fn series_dialog(
    ctx: &egui::Context,
    state: &mut UiState,
    connections: &[ConnectionView],
    subscriptions: &[MqttSubscription],
) -> Option<SeriesDraft> {
    let draft = state.series_draft.as_mut()?;

    let mut close = false;
    let mut save = false;

    egui::Window::new(if draft.is_new {
        "New data series"
    } else {
        "Edit data series"
    })
    .collapsible(false)
    .resizable(false)
    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
    .show(ctx, |ui| {
        ui.set_width(WIDTH);

        ui.label("Name");
        ui.add(
            egui::TextEdit::singleline(&mut draft.series.name)
                .hint_text("温度")
                .desired_width(f32::INFINITY),
        );

        ui.label("Connection");
        egui::ComboBox::from_id_salt("series-connection")
            .selected_text(connection_name(connections, &draft.series.connection_id))
            .width(WIDTH)
            .show_ui(ui, |ui| {
                for connection in connections {
                    ui.selectable_label(
                        draft.series.connection_id == connection.config.id,
                        connection.config.name.clone(),
                    )
                    .clicked()
                    .then(|| draft.series.connection_id = connection.config.id.clone());
                }
            });

        ui.label("Topic");
        // The topics of the subscriptions of the connection are offered, but
        // any topic filter can be typed.
        let topics = topics_of(subscriptions, &draft.series.connection_id);
        egui::ComboBox::from_id_salt("series-topic")
            .selected_text(draft.series.topic.clone())
            .width(WIDTH)
            .show_ui(ui, |ui| {
                for topic in &topics {
                    ui.selectable_label(&draft.series.topic == topic, topic)
                        .clicked()
                        .then(|| draft.series.topic = topic.clone());
                }
            });
        ui.add(
            egui::TextEdit::singleline(&mut draft.series.topic)
                .hint_text("sensors/room/#")
                .desired_width(f32::INFINITY),
        );

        ui.label("Field path");
        ui.add(
            egui::TextEdit::singleline(&mut draft.series.path)
                .hint_text("data.temperature")
                .desired_width(f32::INFINITY),
        );
        ui.label(
            RichText::new("字段路径用点分隔，数组写作 [0]；留空表示整个负载（负载要是一个数字）")
                .small()
                .color(ui.visuals().weak_text_color()),
        );

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
        state.series_draft.take()
    } else {
        if close {
            state.series_draft = None;
        }
        None
    }
}

/// The name of a connection, or a placeholder when it is gone.
fn connection_name(connections: &[ConnectionView], connection_id: &str) -> String {
    connections
        .iter()
        .find(|connection| connection.config.id == connection_id)
        .map(|connection| connection.config.name.clone())
        .unwrap_or_else(|| "（连接已删除）".to_string())
}

/// The topics of the subscriptions of one connection.
fn topics_of(subscriptions: &[MqttSubscription], connection_id: &str) -> Vec<String> {
    let mut topics: Vec<String> = subscriptions
        .iter()
        .filter(|subscription| subscription.connection_id == connection_id)
        .map(|subscription| subscription.topic.clone())
        .collect();
    topics.sort();
    topics.dedup();
    topics
}
