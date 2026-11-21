//! Shared helpers for the engine integration tests.
//!
//! Every test starts its own broker on a free port, so the tests are
//! independent and do not need a broker running on the machine.

#![allow(dead_code)]

use std::collections::HashMap;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use mqtt_core::models::{ConnectionStatus, MqttConnection, MqttMessage, MqttVersion};
use mqtt_engine::{MqttEngine, MqttEvent};
use rumqttd::{Broker, Config, ConnectionSettings, RouterConfig, ServerSettings};

/// A broker listening for MQTT clients.
pub struct BrokerHandle {
    pub port: u16,
}

impl BrokerHandle {
    /// Start a broker that speaks MQTT 3.1.1.
    pub fn start_v4() -> BrokerHandle {
        let port = free_port();
        let mut v4 = HashMap::new();
        v4.insert("v4".to_string(), server_settings(port));
        let config = Config {
            id: 0,
            router: router_config(),
            v4: Some(v4),
            v5: None,
            ws: None,
            cluster: None,
            console: None,
            bridge: None,
            prometheus: None,
            metrics: None,
        };
        spawn_broker(config);
        wait_for_port(port);
        BrokerHandle { port }
    }

    /// Start a broker that speaks MQTT 5.0.
    pub fn start_v5() -> BrokerHandle {
        let port = free_port();
        let mut v5 = HashMap::new();
        v5.insert("v5".to_string(), server_settings(port));
        let config = Config {
            id: 0,
            router: router_config(),
            v4: None,
            v5: Some(v5),
            ws: None,
            cluster: None,
            console: None,
            bridge: None,
            prometheus: None,
            metrics: None,
        };
        spawn_broker(config);
        wait_for_port(port);
        BrokerHandle { port }
    }

    /// A connection configuration pointing at this broker.
    pub fn connection(&self, id: &str, version: MqttVersion) -> MqttConnection {
        let mut config = MqttConnection::new(id, id, "127.0.0.1", self.port);
        config.protocol = version;
        config.client_id = format!("client-{id}");
        config.auto_reconnect = false;
        config
    }
}

/// An engine plus the events it has produced so far.
///
/// Events are accumulated rather than dropped, so that a test can wait for one
/// event without losing the others it saw on the way.
pub struct Harness {
    pub engine: MqttEngine,
    events: Vec<MqttEvent>,
}

impl Harness {
    pub fn new() -> Harness {
        Harness {
            engine: MqttEngine::new(),
            events: Vec::new(),
        }
    }

    fn pump(&mut self) {
        self.events.extend(self.engine.poll_events(512));
    }

    /// Wait until an event satisfies the predicate.
    pub fn wait_for<F>(&mut self, predicate: F) -> MqttEvent
    where
        F: Fn(&MqttEvent) -> bool,
    {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            self.pump();
            if let Some(found) = self.events.iter().find(|event| predicate(event)) {
                return found.clone();
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("no matching event arrived; saw {:?}", self.events);
    }

    /// Wait until the engine reports the given status for a connection.
    pub fn wait_for_status(&mut self, connection_id: &str, status: ConnectionStatus) {
        self.wait_for(|event| {
            matches!(
                event,
                MqttEvent::ConnectionStatusChanged { connection_id: id, status: changed }
                    if id == connection_id && *changed == status
            )
        });
    }

    /// Wait until a message arrives on the given topic.
    pub fn wait_for_message(&mut self, topic: &str) -> MqttMessage {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            self.pump();
            if let Some(message) = self.events.iter().find_map(|event| match event {
                MqttEvent::MessageReceived(message) if message.topic == topic => {
                    Some(message.clone())
                }
                _ => None,
            }) {
                return message;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("no message arrived on topic {topic}");
    }

    /// Drain pending engine events into the accumulated log.
    pub fn pump_public(&mut self) {
        self.pump();
    }

    /// All events seen so far.
    pub fn events(&self) -> &[MqttEvent] {
        &self.events
    }
}

fn router_config() -> RouterConfig {
    RouterConfig {
        max_connections: 100,
        max_outgoing_packet_count: 200,
        max_segment_size: 104_857_600,
        max_segment_count: 10,
        custom_segment: None,
        initialized_filters: None,
        shared_subscriptions_strategy: Default::default(),
    }
}

fn server_settings(port: u16) -> ServerSettings {
    ServerSettings {
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
    }
}

fn spawn_broker(config: Config) {
    let mut broker = Broker::new(config);
    std::thread::spawn(move || {
        let _ = broker.start();
    });
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("can bind a port");
    listener.local_addr().expect("has an address").port()
}

fn wait_for_port(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the broker did not start listening on port {port}");
}
