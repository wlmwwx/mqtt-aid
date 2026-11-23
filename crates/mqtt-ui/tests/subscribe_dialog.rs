//! Headless tests of the subscribe form.

use egui::{Context, RawInput};
use mqtt_core::models::{ConnectionStatus, MqttConnection, MqttVersion};
use mqtt_ui::dialogs::subscribe_dialog;
use mqtt_ui::state::{ConnectionView, SubscribeDraft, UiState};

/// A connection as the form sees it.
fn connection(id: &str, name: &str) -> ConnectionView {
    let mut config = MqttConnection::new(id, name, "127.0.0.1", 1883);
    config.protocol = MqttVersion::V311;
    ConnectionView {
        config,
        status: ConnectionStatus::Connected,
    }
}

/// Draw the form a few times and return the texts it shows.
///
/// A floating window is laid out invisible in its first frame, so the form
/// is drawn again before its texts are looked for.
fn draw(ctx: &Context, state: &mut UiState, connections: &[ConnectionView]) -> Vec<String> {
    let mut texts = Vec::new();
    for _ in 0..3 {
        let output = ctx.run_ui(RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let _ = subscribe_dialog(state, ui.ctx(), connections);
            });
        });
        for shape in &output.shapes {
            if let egui::Shape::Text(text_shape) = &shape.shape {
                texts.push(text_shape.galley.job.text.to_string());
            }
        }
        output.drop_without_applying_deltas();
    }
    texts
}

/// The form opens and names the connection the subscription is made on.
///
/// The connection picker used to have an infinite width, which made the
/// window's rect NaN: egui stopped with a panic and the application went
/// down as soon as the form was opened.
#[test]
fn the_form_opens_and_names_the_connection() {
    let ctx = Context::default();
    let mut state = UiState::default();
    let connections = vec![connection("c1", "Local"), connection("c2", "Public")];
    state.subscribe_draft = Some(SubscribeDraft {
        connection_id: "c2".to_string(),
        topic: String::new(),
        qos: 0,
    });

    let texts = draw(&ctx, &mut state, &connections);

    assert!(
        texts.iter().any(|text| text == "Public"),
        "the form does not name the connection: {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text == "Local"),
        "the form names another connection: {texts:?}"
    );
}
