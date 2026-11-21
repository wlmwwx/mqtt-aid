//! Connection lifecycle at the event-channel seam.

mod common;

use common::{BrokerHandle, Harness};
use mqtt_core::models::{ConnectionStatus, MqttVersion};
use mqtt_engine::{MqttCommand, MqttEvent};

#[test]
fn connects_and_reports_connected() {
    let broker = BrokerHandle::start_v4();
    let mut harness = Harness::new();

    harness
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("primary", MqttVersion::V311),
        ))
        .expect("the engine accepts the command");

    harness.wait_for_status("primary", ConnectionStatus::Connected);
    assert_eq!(
        harness.engine.status("primary"),
        Some(ConnectionStatus::Connected)
    );
}

#[test]
fn disconnects_and_reports_disconnected() {
    let broker = BrokerHandle::start_v4();
    let mut harness = Harness::new();

    harness
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("primary", MqttVersion::V311),
        ))
        .expect("the engine accepts the command");
    harness.wait_for_status("primary", ConnectionStatus::Connected);

    harness
        .engine
        .execute(MqttCommand::Disconnect("primary".to_string()))
        .expect("the engine accepts the command");

    harness.wait_for_status("primary", ConnectionStatus::Disconnected);
}

#[test]
fn reports_an_error_when_the_broker_is_unreachable() {
    let mut harness = Harness::new();
    // Port 1 is not going to have a broker on it.
    let mut config = mqtt_core::models::MqttConnection::new("broken", "broken", "127.0.0.1", 1);
    config.auto_reconnect = false;

    harness
        .engine
        .execute(MqttCommand::Connect(config))
        .expect("the engine accepts the command");

    harness.wait_for(
        |event| matches!(event, MqttEvent::Error { connection_id: Some(id), .. } if id == "broken"),
    );
    assert_eq!(
        harness.engine.status("broken"),
        Some(ConnectionStatus::Error)
    );
}

#[test]
fn connects_over_mqtt_5() {
    let broker = BrokerHandle::start_v5();
    let mut harness = Harness::new();

    harness
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("modern", MqttVersion::V5),
        ))
        .expect("the engine accepts the command");

    harness.wait_for_status("modern", ConnectionStatus::Connected);
}

#[test]
fn keeps_connections_independent() {
    let broker = BrokerHandle::start_v4();
    let mut harness = Harness::new();

    harness
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("first", MqttVersion::V311),
        ))
        .expect("the engine accepts the command");
    harness
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("second", MqttVersion::V311),
        ))
        .expect("the engine accepts the command");

    harness.wait_for_status("first", ConnectionStatus::Connected);
    harness.wait_for_status("second", ConnectionStatus::Connected);

    harness
        .engine
        .execute(MqttCommand::Disconnect("first".to_string()))
        .expect("the engine accepts the command");
    harness.wait_for_status("first", ConnectionStatus::Disconnected);

    assert_eq!(
        harness.engine.status("second"),
        Some(ConnectionStatus::Connected)
    );
}
