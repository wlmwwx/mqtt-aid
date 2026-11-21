//! End to end test of the application layer: state, engine and storage.

use std::collections::HashMap;
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use mqtt_app::AppState;
use mqtt_core::models::{MqttConnection, MqttSubscription, MqttVersion};
use mqtt_engine::{MqttCommand, MqttEngine};
use mqtt_ui::ExportFormat;

/// Start a broker on a free port.
fn start_broker() -> u16 {
    use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("can bind a port");
    let port = listener.local_addr().expect("has an address").port();
    drop(listener);

    let settings = ServerSettings {
        name: "test".to_string(),
        listen: format!("127.0.0.1:{port}").parse().expect("valid address"),
        tls: None,
        next_connection_delay_ms: 1,
        connections: ConnectionSettings {
            connection_timeout_ms: 60_000,
            max_payload_size: 16 * 1024 * 1024,
            max_inflight_count: 100,
            auth: None,
            external_auth: None,
            dynamic_filters: true,
        },
    };

    let mut v4 = HashMap::new();
    v4.insert("v4".to_string(), settings);
    let config = Config {
        id: 0,
        router: RouterConfig {
            max_connections: 100,
            max_outgoing_packet_count: 200,
            max_segment_size: 104_857_600,
            max_segment_count: 10,
            custom_segment: None,
            initialized_filters: None,
            shared_subscriptions_strategy: Default::default(),
        },
        v4: Some(v4),
        v5: None,
        ws: None,
        cluster: None,
        console: None,
        bridge: None,
        prometheus: None,
        metrics: None,
    };

    let mut broker = Broker::new(config);
    std::thread::spawn(move || {
        let _ = broker.start();
    });

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return port;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the broker did not start");
}

/// A throwaway configuration directory for one test.
fn temporary_config(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("mqtt-assistant-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("can create the directory");
    dir
}

#[test]
fn captures_messages_from_a_broker() {
    let port = start_broker();
    let mut state = AppState::open_in(temporary_config("capture"));

    let mut connection = MqttConnection::new("local", "Local", "127.0.0.1", port);
    connection.protocol = MqttVersion::V311;
    connection.client_id = "app-test-subscriber".to_string();
    connection.auto_reconnect = false;
    state.save_connection(connection.clone());

    let subscription = MqttSubscription::new("sub-1", "local", "app/#", 1);
    state.save_subscription(subscription.clone());

    state
        .engine
        .execute(MqttCommand::Connect(connection))
        .expect("the engine accepts the command");
    state
        .engine
        .execute(MqttCommand::Subscribe(
            mqtt_core::models::SubscribeRequest {
                connection_id: "local".to_string(),
                id: subscription.id.clone(),
                topic: subscription.topic.clone(),
                qos: subscription.qos,
            },
        ))
        .expect("the engine accepts the command");

    // A second client publishes into the topic the application listens to.
    let publisher = MqttEngine::new();
    let mut publisher_connection = MqttConnection::new("pub", "Publisher", "127.0.0.1", port);
    publisher_connection.client_id = "app-test-publisher".to_string();
    publisher_connection.auto_reconnect = false;
    publisher
        .execute(MqttCommand::Connect(publisher_connection))
        .expect("the engine accepts the command");

    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && !state.statuses.contains_key("local") {
        state.pump_events();
        std::thread::sleep(Duration::from_millis(20));
    }

    publisher
        .execute(MqttCommand::Publish(mqtt_core::models::PublishRequest {
            connection_id: "pub".to_string(),
            topic: "app/telemetry".to_string(),
            payload: b"{\"value\":42}".to_vec(),
            qos: 1,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");

    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        state.pump_events();
        if let Some(message) = state.messages.back()
            && message.topic == "app/telemetry"
        {
            assert_eq!(message.payload, b"{\"value\":42}");
            assert_eq!(state.received, 1);
            publisher.shutdown();
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    publisher.shutdown();
    panic!("the message never arrived");
}

#[test]
fn persists_connections_and_subscriptions() {
    let port = start_broker();
    let directory = temporary_config("persist");
    let mut state = AppState::open_in(directory.clone());

    let mut connection = MqttConnection::new("persisted", "Persisted", "127.0.0.1", port);
    connection.protocol = MqttVersion::V5;
    state.save_connection(connection.clone());
    state.save_subscription(MqttSubscription::new("sub-p", "persisted", "a/#", 2));

    // A fresh state reads the same database back.
    let reloaded = AppState::open_in(directory);
    assert_eq!(reloaded.connections.len(), 1);
    assert_eq!(reloaded.connections[0], connection);
    assert_eq!(reloaded.subscriptions.len(), 1);
    assert_eq!(reloaded.subscriptions[0].topic, "a/#");
    assert_eq!(reloaded.subscriptions[0].qos, 2);
}

#[test]
fn filters_messages_by_topic_and_qos() {
    let mut state = AppState::open_in(temporary_config("filter"));
    state.ui.search = "sensor".to_string();
    state.ui.filter_qos = Some(1);

    let message = mqtt_core::models::MqttMessage {
        id: "m1".to_string(),
        connection_id: "local".to_string(),
        topic: "sensor/1".to_string(),
        payload: b"x".to_vec(),
        qos: 1,
        retain: false,
        received_at: 0,
        version: MqttVersion::V311,
        properties: None,
    };
    state.messages.push_back(message.clone());

    let mut other = message.clone();
    other.id = "m2".to_string();
    other.topic = "other/1".to_string();
    state.messages.push_back(other);

    let mut wrong_qos = message;
    wrong_qos.id = "m3".to_string();
    wrong_qos.qos = 0;
    state.messages.push_back(wrong_qos);

    let visible = state.visible_messages();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].topic, "sensor/1");
}

#[test]
fn exports_messages_in_every_format() {
    let mut state = AppState::open_in(temporary_config("export"));
    state.messages.push_back(mqtt_core::models::MqttMessage {
        id: "m1".to_string(),
        connection_id: "local".to_string(),
        topic: "sensor/1".to_string(),
        payload: vec![0x01, 0x02],
        qos: 0,
        retain: false,
        received_at: 1,
        version: MqttVersion::V311,
        properties: None,
    });

    let json = String::from_utf8(state.export_bytes(ExportFormat::Json)).expect("valid utf8");
    assert!(json.contains("\"topic\": \"sensor/1\""));
    assert!(json.contains("01 02"), "the hex payload is included");

    let csv = String::from_utf8(state.export_bytes(ExportFormat::Csv)).expect("valid utf8");
    assert!(csv.starts_with("time,topic,qos,retain,size,payload_hex"));
    assert!(csv.contains("sensor/1,0,false,2,01 02"));

    let binary = state.export_bytes(ExportFormat::Binary);
    assert_eq!(binary, vec![0x01, 0x02, b'\n']);
}
