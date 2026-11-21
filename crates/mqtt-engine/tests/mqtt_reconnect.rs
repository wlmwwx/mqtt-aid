//! Automatic reconnect (acceptance criterion AC04).
//!
//! A tiny TCP proxy sits between the engine and the broker. Cutting the proxy
//! severs the connection without touching the broker, so the engine has to
//! notice the loss, back off, reconnect and restore its subscriptions.

mod common;

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use common::{BrokerHandle, Harness};
use mqtt_core::models::{MqttVersion, PublishRequest, SubscribeRequest};
use mqtt_engine::{MqttCommand, MqttEvent};

/// A TCP proxy that can be cut and re-established.
struct Proxy {
    port: u16,
    clients: Arc<Mutex<Vec<TcpStream>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Proxy {
    /// Start a proxy in front of `target_port`.
    fn start(target_port: u16) -> Proxy {
        let listener = TcpListener::bind("127.0.0.1:0").expect("can bind a port");
        let port = listener.local_addr().expect("has an address").port();
        let clients: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));

        let handle = {
            let clients = clients.clone();
            let stop = stop.clone();
            listener
                .set_nonblocking(true)
                .expect("the listener can be non-blocking");

            std::thread::spawn(move || {
                loop {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let incoming = match listener.accept() {
                        Ok(incoming) => incoming,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(20));
                            continue;
                        }
                        Err(_) => break,
                    };
                    let mut client = incoming.0;
                    if client.set_nonblocking(false).is_err() {
                        continue;
                    }
                    let Ok(mut broker) = TcpStream::connect(("127.0.0.1", target_port)) else {
                        continue;
                    };

                    clients
                        .lock()
                        .expect("the lock is free")
                        .push(client.try_clone().expect("can clone"));

                    // Broker -> client.
                    let mut reader = broker.try_clone().expect("can clone");
                    let mut writer = client.try_clone().expect("can clone");
                    std::thread::spawn(move || {
                        let mut buffer = [0u8; 4096];
                        loop {
                            match reader.read(&mut buffer) {
                                Ok(0) | Err(_) => break,
                                Ok(count) => {
                                    if writer.write_all(&buffer[..count]).is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    });

                    // Client -> broker.
                    let mut buffer = [0u8; 4096];
                    loop {
                        match client.read(&mut buffer) {
                            Ok(0) | Err(_) => break,
                            Ok(count) => {
                                if broker.write_all(&buffer[..count]).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            })
        };

        Proxy {
            port,
            clients,
            stop,
            handle: Some(handle),
        }
    }

    /// The address the engine should connect to.
    fn address(&self) -> SocketAddr {
        format!("127.0.0.1:{}", self.port)
            .parse()
            .expect("valid address")
    }

    /// Sever every connection without stopping the listener.
    fn cut(&self) {
        for client in self.clients.lock().expect("the lock is free").drain(..) {
            let _ = client.shutdown(Shutdown::Both);
        }
    }
}

impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.cut();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[test]
fn reconnects_and_restores_the_subscriptions() {
    let broker = BrokerHandle::start_v4();
    let proxy = Proxy::start(broker.port);

    let mut harness = Harness::new();
    let mut config = broker.connection("primary", MqttVersion::V311);
    config.host = proxy.address().ip().to_string();
    config.port = proxy.address().port();
    config.auto_reconnect = true;

    harness
        .engine
        .execute(MqttCommand::Connect(config))
        .expect("the engine accepts the command");
    harness.wait_for_status("primary", mqtt_core::models::ConnectionStatus::Connected);

    harness
        .engine
        .execute(MqttCommand::Subscribe(SubscribeRequest {
            connection_id: "primary".to_string(),
            id: "sub-1".to_string(),
            topic: "reconnect/#".to_string(),
            qos: 1,
        }))
        .expect("the engine accepts the command");
    harness.wait_for(|event| {
        matches!(event, MqttEvent::SubscriptionAcknowledged { topic, .. } if topic == "reconnect/#")
    });

    // Cut the connection: the engine has to notice and start reconnecting.
    proxy.cut();
    harness.wait_for(|event| {
        matches!(
            event,
            MqttEvent::ConnectionStatusChanged {
                connection_id,
                status: mqtt_core::models::ConnectionStatus::Reconnecting,
            } if connection_id == "primary"
        )
    });

    // The proxy is still listening, so the reconnect succeeds.
    harness.wait_for_status("primary", mqtt_core::models::ConnectionStatus::Connected);

    // The subscription is restored: the broker acknowledges it a second time.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        harness.pump_public();
        let acknowledgements = harness
            .events()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    MqttEvent::SubscriptionAcknowledged { topic, .. } if topic == "reconnect/#"
                )
            })
            .count();
        if acknowledgements >= 2 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the subscription was not restored after the reconnect"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    // The subscription must have been restored on the new session.
    let mut publisher = Harness::new();
    publisher
        .engine
        .execute(MqttCommand::Connect(
            broker.connection("publisher", MqttVersion::V311),
        ))
        .expect("the engine accepts the command");
    publisher.wait_for_status("publisher", mqtt_core::models::ConnectionStatus::Connected);

    publisher
        .engine
        .execute(MqttCommand::Publish(PublishRequest {
            connection_id: "publisher".to_string(),
            topic: "reconnect/after".to_string(),
            payload: b"still here".to_vec(),
            qos: 1,
            retain: false,
            properties: None,
        }))
        .expect("the engine accepts the command");

    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        harness.pump_public();
        if harness.events().iter().any(|event| {
            matches!(event, MqttEvent::MessageReceived(message) if message.topic == "reconnect/after")
        }) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    panic!("the message did not arrive after the reconnect");
}
