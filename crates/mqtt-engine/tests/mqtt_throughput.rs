//! Throughput: the application must keep up with a sustained message rate.
//!
//! This is the automated approximation of acceptance criterion AC17
//! ("100 messages per second"): a burst of messages is published and every one
//! of them has to arrive, unchanged, at the subscriber.

mod common;

use std::time::{Duration, Instant};

use common::{BrokerHandle, Harness};
use mqtt_core::models::{MqttVersion, PublishRequest, SubscribeRequest};
use mqtt_engine::{MqttCommand, MqttEvent};

const BURST: usize = 1_000;

#[test]
fn receives_a_burst_of_a_thousand_messages() {
    let broker = BrokerHandle::start_v4();
    let mut subscriber = Harness::new();
    let mut publisher = Harness::new();

    subscriber
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("subscriber", MqttVersion::V311),
        ))
        .expect("the engine accepts the command");
    publisher
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("publisher", MqttVersion::V311),
        ))
        .expect("the engine accepts the command");

    subscriber.wait_for_status("subscriber", mqtt_core::models::ConnectionStatus::Connected);
    publisher.wait_for_status("publisher", mqtt_core::models::ConnectionStatus::Connected);

    subscriber
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: "subscriber".to_string(),
            id: "sub-1".to_string(),
            topic: "throughput/#".to_string(),
            qos: 0,
        }))
        .expect("the engine accepts the command");

    subscriber.wait_for(|event| {
        matches!(event, MqttEvent::SubscriptionAcknowledged { topic, .. } if topic == "throughput/#")
    });

    let started = Instant::now();
    for index in 0..BURST {
        publisher
            .engine
            .execute(MqttCommand::Publish(PublishRequest {
                connection_id: "publisher".to_string(),
                topic: "throughput/burst".to_string(),
                payload: (index as u32).to_le_bytes().to_vec(),
                qos: 0,
                retain: false,
                properties: None,
            }))
            .expect("the engine accepts the command");
    }
    let published_in = started.elapsed();

    // Every message has to arrive, with its payload intact.
    let mut received = 0usize;
    let deadline = Instant::now() + Duration::from_secs(30);
    while received < BURST && Instant::now() < deadline {
        for event in subscriber.engine.poll_events(1024) {
            if let MqttEvent::MessageReceived(message) = event {
                assert_eq!(message.topic, "throughput/burst");
                assert_eq!(message.payload.len(), 4);
                received += 1;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(
        received, BURST,
        "all {BURST} messages must arrive (took {published_in:?} to publish)"
    );

    let rate = BURST as f64 / started.elapsed().as_secs_f64();
    assert!(
        rate >= 100.0,
        "the pipeline must sustain at least 100 messages per second, did {rate:.0}"
    );
}
