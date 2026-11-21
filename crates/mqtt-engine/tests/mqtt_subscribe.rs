//! Subscribing and receiving messages at the event-channel seam.

mod common;

use common::{BrokerHandle, Harness};
use mqtt_core::models::{MqttVersion, SubscribeRequest};
use mqtt_engine::{MqttCommand, MqttEvent};

/// Connect a subscriber and a publisher to the same broker.
fn pair(version: MqttVersion) -> (BrokerHandle, Harness, Harness) {
    let broker = BrokerHandle::start_v4();
    let mut subscriber = Harness::new();
    let mut publisher = Harness::new();

    subscriber
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("subscriber", version),
        ))
        .expect("the engine accepts the command");
    publisher
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("publisher", version),
        ))
        .expect("the engine accepts the command");

    subscriber.wait_for_status("subscriber", mqtt_core::models::ConnectionStatus::Connected);
    publisher.wait_for_status("publisher", mqtt_core::models::ConnectionStatus::Connected);

    (broker, subscriber, publisher)
}

#[test]
fn receives_a_message_on_an_exact_topic() {
    let (_broker, mut subscriber, publisher) = pair(MqttVersion::V311);

    subscriber
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: "subscriber".to_string(),
            id: "sub-1".to_string(),
            topic: "devices/001/status".to_string(),
            qos: 1,
        }))
        .expect("the engine accepts the command");

    subscriber.wait_for(|event| {
        matches!(
            event,
            MqttEvent::SubscriptionAcknowledged { topic, .. } if topic == "devices/001/status"
        )
    });

    publisher
        .engine
        .execute(MqttCommand::Publish(mqtt_core::models::PublishRequest {
            connection_id: "publisher".to_string(),
            topic: "devices/001/status".to_string(),
            payload: b"online".to_vec(),
            qos: 1,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");

    let message = subscriber.wait_for_message("devices/001/status");
    assert_eq!(message.payload, b"online");
    assert_eq!(message.connection_id, "subscriber");
    assert_eq!(message.qos, 1);
    assert!(!message.retain);
}

#[test]
fn receives_a_message_through_a_single_level_wildcard() {
    let (_broker, mut subscriber, publisher) = pair(MqttVersion::V311);

    subscriber
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: "subscriber".to_string(),
            id: "sub-1".to_string(),
            topic: "devices/+/status".to_string(),
            qos: 0,
        }))
        .expect("the engine accepts the command");

    subscriber.wait_for(|event| {
        matches!(
            event,
            MqttEvent::SubscriptionAcknowledged { topic, .. } if topic == "devices/+/status"
        )
    });

    publisher
        .engine
        .execute(MqttCommand::Publish(mqtt_core::models::PublishRequest {
            connection_id: "publisher".to_string(),
            topic: "devices/042/status".to_string(),
            payload: b"offline".to_vec(),
            qos: 0,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");

    let message = subscriber.wait_for_message("devices/042/status");
    assert_eq!(message.payload, b"offline");
}

#[test]
fn receives_a_message_through_a_multi_level_wildcard() {
    let (_broker, mut subscriber, publisher) = pair(MqttVersion::V311);

    subscriber
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: "subscriber".to_string(),
            id: "sub-1".to_string(),
            topic: "sensors/#".to_string(),
            qos: 0,
        }))
        .expect("the engine accepts the command");

    subscriber.wait_for(|event| {
        matches!(event, MqttEvent::SubscriptionAcknowledged { topic, .. } if topic == "sensors/#")
    });

    publisher
        .engine
        .execute(MqttCommand::Publish(mqtt_core::models::PublishRequest {
            connection_id: "publisher".to_string(),
            topic: "sensors/room/temperature".to_string(),
            payload: vec![0x01, 0x02, 0xff],
            qos: 0,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");

    let message = subscriber.wait_for_message("sensors/room/temperature");
    assert_eq!(message.payload, vec![0x01, 0x02, 0xff]);
}

#[test]
fn stops_receiving_after_unsubscribing() {
    let (_broker, mut subscriber, publisher) = pair(MqttVersion::V311);

    subscriber
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: "subscriber".to_string(),
            id: "sub-1".to_string(),
            topic: "news/#".to_string(),
            qos: 0,
        }))
        .expect("the engine accepts the command");

    subscriber.wait_for(|event| {
        matches!(event, MqttEvent::SubscriptionAcknowledged { topic, .. } if topic == "news/#")
    });

    subscriber
        .engine
        .execute(MqttCommand::Unsubscribe {
            connection_id: "subscriber".to_string(),
            topic: "news/#".to_string(),
        })
        .expect("the engine accepts the command");

    // Give the unsubscribe time to reach the broker before publishing.
    std::thread::sleep(std::time::Duration::from_millis(300));

    publisher
        .engine
        .execute(MqttCommand::Publish(mqtt_core::models::PublishRequest {
            connection_id: "publisher".to_string(),
            topic: "news/today".to_string(),
            payload: b"ignored".to_vec(),
            qos: 0,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");

    std::thread::sleep(std::time::Duration::from_millis(500));
    subscriber.pump_public();

    let received = subscriber
        .events()
        .iter()
        .any(|event| matches!(event, MqttEvent::MessageReceived(message) if message.topic == "news/today"));
    assert!(!received, "the message must not arrive after unsubscribing");
}

#[test]
fn receives_binary_payloads_byte_for_byte() {
    let (_broker, mut subscriber, publisher) = pair(MqttVersion::V311);

    let payload: Vec<u8> = (0u8..=255).collect();

    subscriber
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: "subscriber".to_string(),
            id: "sub-1".to_string(),
            topic: "binary/blob".to_string(),
            qos: 1,
        }))
        .expect("the engine accepts the command");

    subscriber.wait_for(|event| {
        matches!(event, MqttEvent::SubscriptionAcknowledged { topic, .. } if topic == "binary/blob")
    });

    publisher
        .engine
        .execute(MqttCommand::Publish(mqtt_core::models::PublishRequest {
            connection_id: "publisher".to_string(),
            topic: "binary/blob".to_string(),
            payload: payload.clone(),
            qos: 1,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");

    let message = subscriber.wait_for_message("binary/blob");
    assert_eq!(message.payload, payload);
}
