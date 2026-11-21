//! Tests of the commands the menu bar and the ribbon can trigger.
//!
//! The commands are the vocabulary the UI speaks: they arrive at the
//! application layer and have to turn into engine commands, into state
//! changes or into something the user sees.

use std::path::PathBuf;

use mqtt_app::AppState;
use mqtt_core::models::{MqttConnection, PayloadFormat};

/// A throwaway configuration directory for one test.
fn temporary_config(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "mqtt-assistant-command-test-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("can create the directory");
    dir
}

/// A state with one saved connection.
fn state_with_connection(name: &str) -> AppState {
    let mut state = AppState::open_in(temporary_config(name));
    let connection = MqttConnection::new("local", "Local", "127.0.0.1", 1883);
    state.save_connection(connection);
    state.ui.selected_connection = Some("local".to_string());
    state
}

#[test]
fn formatting_the_publish_payload_indents_it() {
    let mut state = state_with_connection("format");
    state.ui.publish.connection_id = Some("local".to_string());
    state.ui.publish.payload = "{\"a\":1}".to_string();
    state.ui.publish.json_indent = 4;

    state.format_publish_json(true);

    assert_eq!(state.ui.publish.payload, "{\n    \"a\": 1\n}");
    assert!(state.ui.publish.validation_error.is_none());
}

#[test]
fn minifying_the_publish_payload_compacts_it() {
    let mut state = state_with_connection("minify");
    state.ui.publish.connection_id = Some("local".to_string());
    state.ui.publish.payload = "{\n  \"a\": 1\n}".to_string();

    state.format_publish_json(false);

    assert_eq!(state.ui.publish.payload, "{\"a\":1}");
}

#[test]
fn an_invalid_json_payload_reports_the_error_and_keeps_the_text() {
    let mut state = state_with_connection("invalid-json");
    state.ui.publish.connection_id = Some("local".to_string());
    state.ui.publish.payload = "{\"a\":".to_string();

    state.format_publish_json(true);

    assert_eq!(state.ui.publish.payload, "{\"a\":");
    assert!(state.ui.publish.validation_error.is_some());
}

#[test]
fn the_publish_form_becomes_a_request() {
    let mut state = state_with_connection("request");
    state.ui.publish.connection_id = Some("local".to_string());
    state.ui.publish.topic = "  device/001/cmd  ".to_string();
    state.ui.publish.payload = "hello".to_string();
    state.ui.publish.qos = 2;
    state.ui.publish.retain = true;

    let request = state.publish_request().expect("the form is ready");

    assert_eq!(request.connection_id, "local");
    assert_eq!(request.topic, "device/001/cmd");
    assert_eq!(request.payload, b"hello");
    assert_eq!(request.qos, 2);
    assert!(request.retain);
}

#[test]
fn an_empty_publish_form_is_not_a_request() {
    let state = state_with_connection("empty-request");

    assert!(state.publish_request().is_none());
}

#[test]
fn a_hex_payload_is_encoded_into_bytes() {
    let mut state = state_with_connection("hex-request");
    state.ui.publish.connection_id = Some("local".to_string());
    state.ui.publish.topic = "device/001/cmd".to_string();
    state.ui.publish.payload = "01 03 00".to_string();
    state.ui.publish.format = PayloadFormat::Hex;

    let request = state.publish_request().expect("the form is ready");

    assert_eq!(request.payload, vec![0x01, 0x03, 0x00]);
}

#[test]
fn the_publish_form_becomes_a_template() {
    let mut state = state_with_connection("template");
    state.ui.publish.connection_id = Some("local".to_string());
    state.ui.publish.topic = "device/001/cmd".to_string();
    state.ui.publish.payload = "hello".to_string();

    let template = state.publish_template().expect("the form is ready");

    assert_eq!(template.topic, "device/001/cmd");
    assert_eq!(template.payload, b"hello");
    assert!(template.name.contains("device/001/cmd"));
}
