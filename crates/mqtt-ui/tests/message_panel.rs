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

/// The scroll bar of the table belongs at the right edge of the panel.
///
/// A scroll area shrinks to the width of its content by default, and then
/// its scroll bar ends up in the middle of the panel, where it looks like
/// it belongs to the panel on the right.
#[test]
fn the_scroll_bar_of_the_table_is_at_the_edge_of_the_panel() {
    let ctx = Context::default();
    let mut state = UiState::default();
    let mut actions = UiActions::default();

    let width = 800.0;
    let messages: Vec<MqttMessage> = (0..40)
        .map(|index| MqttMessage {
            id: format!("m{index}"),
            connection_id: "c1".to_string(),
            topic: format!("sensors/room/{index}"),
            payload: b"21.5".to_vec(),
            qos: 0,
            retain: false,
            received_at: 1_700_000_000_000,
            version: MqttVersion::V5,
            properties: None,
        })
        .collect();

    let input = RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width, 600.0),
        )),
        ..Default::default()
    };
    let output = ctx.run_ui(input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            message_panel(ui, &mut state, &messages, &[], &mut actions);
        });
    });

    // The handle of the scroll bar: a small filled rectangle.
    let mut handle = None;
    for shape in &output.shapes {
        if let egui::Shape::Rect(rect) = &shape.shape
            && rect.rect.width() < 14.0
            && rect.rect.height() > 10.0
            && rect.rect.min.x > width / 2.0
        {
            handle = Some(rect.rect);
        }
    }
    output.drop_without_applying_deltas();

    let handle = handle.expect("the table has no scroll bar at the right edge");
    assert!(
        handle.max.x > width - 20.0,
        "the scroll bar is at x={} instead of at the edge of the panel",
        handle.min.x
    );
}

/// The names of the columns stand above the columns themselves.
///
/// The header and the rows are drawn by hand, cell by cell: if one of them
/// moved, the table would be unreadable — a topic that starts under the
/// wrong column.
#[test]
fn the_header_of_the_table_stands_above_its_columns() {
    let ctx = Context::default();
    let mut state = UiState::default();
    let mut actions = UiActions::default();
    let messages = vec![json_message()];

    let input = RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(800.0, 600.0),
        )),
        ..Default::default()
    };
    let output = ctx.run_ui(input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            message_panel(ui, &mut state, &messages, &[], &mut actions);
        });
    });

    let mut places: Vec<(String, egui::Pos2)> = Vec::new();
    for shape in &output.shapes {
        if let egui::Shape::Text(text_shape) = &shape.shape {
            places.push((text_shape.galley.job.text.to_string(), text_shape.pos));
        }
    }
    output.drop_without_applying_deltas();

    let x_of = |needle: &str| {
        places
            .iter()
            .find(|(text, _)| text == needle)
            .map(|(_, place)| place.x)
            .unwrap_or_else(|| panic!("{needle} is not shown"))
    };

    assert_eq!(
        x_of("主题"),
        x_of("sensors/room/temperature"),
        "the topic is not under the name of its column"
    );
    assert_eq!(
        x_of("时间"),
        x_of("11-14 22:13:20"),
        "the time is not under the name of its column"
    );
}
