//! Headless tests of the message monitor.
//!
//! The message table and the detail view of the selected message both live
//! in the same panel. The detail view shows a JSON payload as a tree inside a
//! scroll area, and a scroll area that shares its id with the table makes
//! egui draw a "Second use of ScrollArea ID" error over the window.

use egui::{Context, RawInput};
use mqtt_core::models::{MqttMessage, MqttVersion, PayloadFormat};
use mqtt_ui::panels::message_panel::message_panel;
use mqtt_ui::state::{UiActions, UiState};

/// A message with a JSON payload.
fn json_message() -> MqttMessage {
    MqttMessage {
        id: "m1".to_string(),
        connection_id: "c1".to_string(),
        topic: "sensors/room/temperature".to_string(),
        payload: br#"{"room": "kitchen", "value": 21.5, "unit": "C"}"#.to_vec(),
        qos: 1,
        retain: false,
        received_at: 1_700_000_000_000,
        version: MqttVersion::V5,
        properties: None,
    }
}

/// Render the message monitor the way the application does, in the central
/// panel, and return the text that egui painted.
fn render(state: &mut UiState, messages: &[MqttMessage]) -> String {
    let ctx = Context::default();
    let mut actions = UiActions::default();
    let output = ctx.run_ui(RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            message_panel(ui, state, messages, &[], &mut actions);
        });
    });

    let mut text = String::new();
    for shape in &output.shapes {
        if let egui::Shape::Text(text_shape) = &shape.shape {
            text.push_str(&text_shape.galley.job.text);
            text.push('\n');
        }
    }
    output.drop_without_applying_deltas();
    text
}

#[test]
fn a_json_payload_does_not_clash_with_the_message_table() {
    let mut state = UiState {
        selected_message: Some(0),
        message_format: PayloadFormat::Auto,
        ..Default::default()
    };

    let text = render(&mut state, &[json_message()]);

    assert!(
        !text.contains("use of ScrollArea ID"),
        "the message table and the JSON tree share a scroll area id:\n{text}"
    );
}

#[test]
fn a_json_payload_is_shown_as_a_tree() {
    let mut state = UiState {
        selected_message: Some(0),
        message_format: PayloadFormat::Auto,
        ..Default::default()
    };

    let text = render(&mut state, &[json_message()]);

    assert!(text.contains("room"), "the JSON tree is missing:\n{text}");
    assert!(
        text.contains("kitchen"),
        "the JSON tree is missing:\n{text}"
    );
}

#[test]
fn a_text_payload_is_shown_as_it_is() {
    let mut state = UiState {
        selected_message: Some(0),
        message_format: PayloadFormat::Auto,
        ..Default::default()
    };
    let mut message = json_message();
    message.payload = b"hello".to_vec();

    let text = render(&mut state, &[message]);

    assert!(text.contains("hello"), "the payload is missing:\n{text}");
    assert!(!text.contains("use of ScrollArea ID"));
}
