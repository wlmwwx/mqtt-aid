//! Publishing at the event-channel seam.

mod common;

use common::{BrokerHandle, Harness};
use mqtt_core::models::{MqttProperties, MqttVersion, PublishRequest, SubscribeRequest};
use mqtt_engine::{MqttCommand, MqttEvent};

fn connected(broker: &BrokerHandle, id: &str, version: MqttVersion) -> Harness {
    let mut harness = Harness::new();
    harness
        .engine
        .execute(MqttCommand::Connect(broker.connection(id, version)))
        .expect("the engine accepts the command");
    harness.wait_for_status(id, mqtt_core::models::ConnectionStatus::Connected);
    harness
}

fn subscribe(harness: &mut Harness, id: &str, topic: &str, qos: u8) {
    harness
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: id.to_string(),
            id: format!("sub-{id}"),
            topic: topic.to_string(),
            qos,
        }))
        .expect("the engine accepts the command");
    harness.wait_for(
        |event| matches!(event, MqttEvent::SubscriptionAcknowledged { topic: t, .. } if t == topic),
    );
}

fn publish(harness: &Harness, id: &str, topic: &str, payload: &[u8], qos: u8, retain: bool) {
    harness
        .engine
        .execute(MqttCommand::Publish(PublishRequest {
            connection_id: id.to_string(),
            topic: topic.to_string(),
            payload: payload.to_vec(),
            qos,
            retain,
            properties: None,
        }))
        .expect("the engine accepts the command");
}

#[test]
fn publishes_with_qos_0() {
    let broker = BrokerHandle::start_v4();
    let mut subscriber = connected(&broker, "subscriber", MqttVersion::V311);
    let publisher = connected(&broker, "publisher", MqttVersion::V311);

    subscribe(&mut subscriber, "subscriber", "cmd/one", 0);
    publish(&publisher, "publisher", "cmd/one", b"go", 0, false);

    let message = subscriber.wait_for_message("cmd/one");
    assert_eq!(message.payload, b"go");
    assert_eq!(message.qos, 0);
}

#[test]
fn publishes_with_qos_1() {
    let broker = BrokerHandle::start_v4();
    let mut subscriber = connected(&broker, "subscriber", MqttVersion::V311);
    let publisher = connected(&broker, "publisher", MqttVersion::V311);

    subscribe(&mut subscriber, "subscriber", "cmd/two", 1);
    publish(&publisher, "publisher", "cmd/two", b"go", 1, false);

    let message = subscriber.wait_for_message("cmd/two");
    assert_eq!(message.payload, b"go");
    assert_eq!(message.qos, 1);
}

#[test]
fn publishes_with_qos_2() {
    let broker = BrokerHandle::start_v4();
    let mut subscriber = connected(&broker, "subscriber", MqttVersion::V311);
    let publisher = connected(&broker, "publisher", MqttVersion::V311);

    subscribe(&mut subscriber, "subscriber", "cmd/three", 2);
    publish(&publisher, "publisher", "cmd/three", b"go", 2, false);

    let message = subscriber.wait_for_message("cmd/three");
    assert_eq!(message.payload, b"go");
    assert_eq!(message.qos, 2);
}

#[test]
fn reports_the_publish_as_completed() {
    let broker = BrokerHandle::start_v4();
    let mut publisher = connected(&broker, "publisher", MqttVersion::V311);

    publish(&publisher, "publisher", "cmd/ack", b"go", 0, false);

    publisher.wait_for(
        |event| matches!(event, MqttEvent::PublishCompleted { topic, .. } if topic == "cmd/ack"),
    );
}

#[test]
fn delivers_a_retained_message_to_a_later_subscriber() {
    let broker = BrokerHandle::start_v4();
    let mut publisher = connected(&broker, "publisher", MqttVersion::V311);

    publish(&publisher, "publisher", "state/device", b"on", 1, true);
    publisher.wait_for(|event| {
        matches!(event, MqttEvent::PublishCompleted { topic, .. } if topic == "state/device")
    });

    let mut late = connected(&broker, "late", MqttVersion::V311);
    subscribe(&mut late, "late", "state/device", 1);

    let message = late.wait_for_message("state/device");
    assert_eq!(message.payload, b"on");
    assert!(message.retain, "a retained message keeps the retain flag");
}

#[test]
fn reports_an_error_when_publishing_on_a_closed_connection() {
    let mut harness = Harness::new();

    // Never connected: the engine has no such connection.
    harness
        .engine
        .execute(MqttCommand::Publish(PublishRequest {
            connection_id: "nowhere".to_string(),
            topic: "cmd/one".to_string(),
            payload: b"go".to_vec(),
            qos: 0,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");

    harness.wait_for(|event| {
        matches!(event, MqttEvent::Error { connection_id: Some(id), .. } if id == "nowhere")
    });
}

#[test]
fn carries_mqtt_5_properties() {
    let broker = BrokerHandle::start_v5();
    let mut subscriber = connected(&broker, "subscriber", MqttVersion::V5);
    let publisher = connected(&broker, "publisher", MqttVersion::V5);

    subscribe(&mut subscriber, "subscriber", "cmd/props", 0);

    publisher
        .engine
        .execute(MqttCommand::Publish(PublishRequest {
            connection_id: "publisher".to_string(),
            topic: "cmd/props".to_string(),
            payload: b"{\"a\":1}".to_vec(),
            qos: 0,
            retain: false,
            properties: Some(MqttProperties {
                payload_format_indicator: Some(1),
                message_expiry_interval: Some(60),
                response_topic: Some("cmd/response".to_string()),
                correlation_data: Some(vec![1, 2, 3]),
                user_properties: vec![("source".to_string(), "test".to_string())],
                content_type: Some("application/json".to_string()),
            }),
        }))
        .expect("the engine accepts the command");

    let message = subscriber.wait_for_message("cmd/props");
    let properties = message
        .properties
        .expect("the broker forwards the publish properties");
    assert_eq!(properties.content_type.as_deref(), Some("application/json"));
    assert_eq!(properties.response_topic.as_deref(), Some("cmd/response"));
    assert_eq!(properties.correlation_data, Some(vec![1, 2, 3]));
    assert_eq!(properties.payload_format_indicator, Some(1));
    assert_eq!(properties.message_expiry_interval, Some(60));
    assert_eq!(
        properties.user_properties,
        vec![("source".to_string(), "test".to_string())]
    );
}
