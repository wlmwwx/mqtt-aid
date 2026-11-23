//! End to end test of the application layer: state, engine and storage.

use std::collections::HashMap;
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use mqtt_app::AppState;
use std::collections::VecDeque;

use mqtt_core::models::{
    ConnectionStatus, DataPoint, DataSeries, MqttConnection, MqttSubscription, MqttVersion,
    SubscribeRequest,
};
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

/// Wait until the application reports the given status for a connection.
fn wait_for_status(state: &mut AppState, connection_id: &str, status: ConnectionStatus) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        state.pump_events();
        if state.statuses.get(connection_id) == Some(&status) {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the connection {connection_id} never reached {status:?}");
}

/// Publish until the application receives the message.
///
/// The publish is repeated, because the subscription and the first publish
/// can reach the broker in either order.
fn publish_until_received(state: &mut AppState, publisher: &MqttEngine, topic: &str) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        publish(publisher, topic);
        for _ in 0..10 {
            state.pump_events();
            std::thread::sleep(Duration::from_millis(20));
        }
        if state.messages.iter().any(|message| message.topic == topic) {
            return;
        }
    }
    panic!("no message arrived on {topic}");
}

/// Publish a raw payload with another client.
fn publish_raw(publisher: &MqttEngine, topic: &str, payload: &[u8]) {
    publisher
        .execute(MqttCommand::Publish(mqtt_core::models::PublishRequest {
            connection_id: "pub".to_string(),
            topic: topic.to_string(),
            payload: payload.to_vec(),
            qos: 1,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");
}

/// Publish a message with another client.
fn publish(publisher: &MqttEngine, topic: &str) {
    publisher
        .execute(MqttCommand::Publish(mqtt_core::models::PublishRequest {
            connection_id: "pub".to_string(),
            topic: topic.to_string(),
            payload: b"{\"value\":42}".to_vec(),
            qos: 1,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");
}

/// A connection that is disconnected and connected again has to subscribe
/// again: the broker forgets the subscriptions of a session that ended.
#[test]
fn a_reconnected_connection_subscribes_again() {
    let port = start_broker();
    let mut state = AppState::open_in(temporary_config("reconnect"));

    let mut connection = MqttConnection::new("local", "Local", "127.0.0.1", port);
    connection.protocol = MqttVersion::V311;
    connection.client_id = "app-test-reconnect".to_string();
    connection.auto_reconnect = false;
    state.save_connection(connection.clone());
    state.save_subscription(MqttSubscription::new("sub-1", "local", "app/#", 1));

    // A second client publishes into the topic the application listens to.
    let publisher = MqttEngine::new();
    let mut publisher_connection = MqttConnection::new("pub", "Publisher", "127.0.0.1", port);
    publisher_connection.client_id = "app-test-reconnect-pub".to_string();
    publisher_connection.auto_reconnect = false;
    publisher
        .execute(MqttCommand::Connect(publisher_connection))
        .expect("the engine accepts the command");

    state
        .engine
        .execute(MqttCommand::Connect(connection.clone()))
        .expect("the engine accepts the command");
    wait_for_status(&mut state, "local", ConnectionStatus::Connected);

    // The user subscribes to a topic.
    state
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: "local".to_string(),
            id: "sub-1".to_string(),
            topic: "app/#".to_string(),
            qos: 1,
        }))
        .expect("the engine accepts the command");

    publish_until_received(&mut state, &publisher, "app/first");

    // The user disconnects and connects again.
    state
        .engine
        .execute(MqttCommand::Disconnect("local".to_string()))
        .expect("the engine accepts the command");
    wait_for_status(&mut state, "local", ConnectionStatus::Disconnected);

    state
        .engine
        .execute(MqttCommand::Connect(connection.clone()))
        .expect("the engine accepts the command");
    wait_for_status(&mut state, "local", ConnectionStatus::Connected);

    // The subscription has to be there again, without the user doing
    // anything.
    publish_until_received(&mut state, &publisher, "app/second");

    publisher.shutdown();
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

/// A data series reads a field out of the messages of a topic and keeps the
/// values as a chart.
#[test]
fn collects_the_values_of_a_data_series() {
    let port = start_broker();
    let mut state = AppState::open_in(temporary_config("series"));

    let mut connection = MqttConnection::new("local", "Local", "127.0.0.1", port);
    connection.protocol = MqttVersion::V311;
    connection.client_id = "app-test-series".to_string();
    connection.auto_reconnect = false;
    state.save_connection(connection.clone());
    state.add_series(DataSeries::new(
        "series-1",
        "温度",
        "local",
        "sensors/room",
        "data.temperature",
    ));

    let publisher = MqttEngine::new();
    let mut publisher_connection = MqttConnection::new("pub", "Publisher", "127.0.0.1", port);
    publisher_connection.client_id = "app-test-series-pub".to_string();
    publisher_connection.auto_reconnect = false;
    publisher
        .execute(MqttCommand::Connect(publisher_connection))
        .expect("the engine accepts the command");

    state
        .engine
        .execute(MqttCommand::Connect(connection.clone()))
        .expect("the engine accepts the command");
    wait_for_status(&mut state, "local", ConnectionStatus::Connected);

    // The first publishes can be lost: the broker may not have registered
    // the subscription yet. So the messages keep coming, with a value that
    // grows every time.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut step = 0u32;
    while Instant::now() < deadline {
        let value = 21.0 + f64::from(step) / 10.0;
        publish_raw(
            &publisher,
            "sensors/room",
            format!("{{\"data\": {{\"temperature\": {value}}}}}").as_bytes(),
        );
        step += 1;

        // A message of another topic is not part of the series, and a
        // message without the field is skipped.
        if step == 3 {
            publish_raw(
                &publisher,
                "sensors/hall",
                br#"{"data": {"temperature": 99.0}}"#,
            );
            publish_raw(&publisher, "sensors/room", br#"{"data": {"humidity": 40}}"#);
        }

        for _ in 0..5 {
            state.pump_events();
            std::thread::sleep(Duration::from_millis(20));
        }

        let count = state
            .samples
            .get("series-1")
            .map_or(0, |points| points.len());
        if count >= 2 {
            break;
        }
    }

    let samples = state
        .samples
        .get("series-1")
        .expect("the series has samples");
    let values: Vec<f64> = samples.iter().map(|point| point.value).collect();
    assert!(values.len() >= 2, "too few values: {values:?}");
    // Every value belongs to this topic and this field, and they arrive in
    // the order the messages were sent.
    assert!(
        values.windows(2).all(|pair| pair[0] < pair[1]),
        "the values are not in order: {values:?}"
    );
    assert!(
        values.iter().all(|value| (21.0..22.0).contains(value)),
        "a value of another topic or field showed up: {values:?}"
    );

    publisher.shutdown();
}

/// A data series survives a restart of the application.
#[test]
fn persists_the_data_series() {
    let directory = temporary_config("series-persist");
    let mut state = AppState::open_in(directory.clone());
    state.add_series(DataSeries::new(
        "series-p",
        "湿度",
        "local",
        "sensors/room",
        "humidity",
    ));

    let reloaded = AppState::open_in(directory);
    assert_eq!(reloaded.series.len(), 1);
    assert_eq!(reloaded.series[0].name, "湿度");
    assert_eq!(reloaded.series[0].topic, "sensors/room");
    assert_eq!(reloaded.series[0].path, "humidity");
}

/// Deleting a connection takes its data series with it.
#[test]
fn forgets_the_series_of_a_deleted_connection() {
    let mut state = AppState::open_in(temporary_config("series-connection"));
    state.add_series(DataSeries::new(
        "series-c",
        "温度",
        "local",
        "sensors/room",
        "temperature",
    ));

    state.delete_connection("local");

    assert!(state.series.is_empty());
}

/// Removing a series takes its samples with it.
#[test]
fn forgets_the_samples_of_a_removed_series() {
    let mut state = AppState::open_in(temporary_config("series-remove"));
    state.add_series(DataSeries::new(
        "series-r",
        "温度",
        "local",
        "sensors/room",
        "temperature",
    ));
    state.samples.insert(
        "series-r".to_string(),
        VecDeque::from([DataPoint { at: 0, value: 1.0 }]),
    );

    state.remove_series("series-r");

    assert!(state.series.is_empty());
    assert!(!state.samples.contains_key("series-r"));
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
