//! The MQTT engine: it owns the background runtime and the broker connections.
//!
//! The engine is the only bridge between the GUI thread and the network. The
//! GUI sends [`MqttCommand`]s and drains [`MqttEvent`]s; it never touches a
//! socket itself.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mqtt_core::models::{ConnectionStatus, MqttConnection};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::command::{ConnCommand, MqttCommand};
use crate::event::MqttEvent;
use crate::session;

/// Default upper bound on simultaneous connections.
pub const DEFAULT_MAX_CONNECTIONS: usize = 16;

/// Something went wrong while talking to the engine.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("the MQTT engine is no longer running")]
    Closed,
}

/// A running MQTT engine.
pub struct MqttEngine {
    runtime: Option<tokio::runtime::Runtime>,
    inner: Arc<Inner>,
}

struct Inner {
    command_tx: UnboundedSender<MqttCommand>,
    events: Mutex<UnboundedReceiver<MqttEvent>>,
    connections: Arc<Mutex<HashMap<String, ConnectionHandle>>>,
    statuses: Arc<Mutex<HashMap<String, ConnectionStatus>>>,
}

struct ConnectionHandle {
    cmd_tx: UnboundedSender<ConnCommand>,
}

impl MqttEngine {
    /// Start the engine with its own background runtime.
    pub fn new() -> Self {
        Self::with_max_connections(DEFAULT_MAX_CONNECTIONS)
    }

    /// Start the engine, limiting how many brokers may be connected at once.
    pub fn with_max_connections(max_connections: usize) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("mqtt-engine")
            .enable_all()
            .build()
            .expect("failed to start the MQTT runtime");

        let (command_tx, command_rx) = unbounded_channel();
        let (event_tx, event_rx) = unbounded_channel();
        let connections: Arc<Mutex<HashMap<String, ConnectionHandle>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let statuses: Arc<Mutex<HashMap<String, ConnectionStatus>>> =
            Arc::new(Mutex::new(HashMap::new()));

        runtime.spawn(dispatcher(
            command_rx,
            connections.clone(),
            statuses.clone(),
            event_tx,
            max_connections,
        ));

        Self {
            runtime: Some(runtime),
            inner: Arc::new(Inner {
                command_tx,
                events: Mutex::new(event_rx),
                connections,
                statuses,
            }),
        }
    }

    /// Hand a command to the engine. Never blocks.
    pub fn execute(&self, command: MqttCommand) -> Result<(), EngineError> {
        self.inner
            .command_tx
            .send(command)
            .map_err(|_| EngineError::Closed)
    }

    /// Drain up to `max` pending events. Never blocks.
    pub fn poll_events(&self, max: usize) -> Vec<MqttEvent> {
        let mut collected = Vec::new();
        let Ok(mut receiver) = self.inner.events.lock() else {
            return collected;
        };
        while collected.len() < max {
            match receiver.try_recv() {
                Ok(event) => collected.push(event),
                Err(_) => break,
            }
        }
        collected
    }

    /// The current status of a connection.
    pub fn status(&self, connection_id: &str) -> Option<ConnectionStatus> {
        self.inner
            .statuses
            .lock()
            .ok()
            .and_then(|statuses| statuses.get(connection_id).copied())
    }

    /// Ids of all connections the engine knows about.
    pub fn connection_ids(&self) -> Vec<String> {
        self.inner
            .connections
            .lock()
            .map(|connections| connections.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Close every connection but keep the engine running.
    pub fn disconnect_all(&self) {
        for id in self.connection_ids() {
            let _ = self.execute(MqttCommand::Disconnect(id));
        }
    }

    /// Stop the engine and release every connection.
    pub fn shutdown(mut self) {
        let ids = self.connection_ids();
        for id in ids {
            let _ = self.execute(MqttCommand::Disconnect(id));
        }
        // Give the connection tasks a moment to close their sockets.
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(std::time::Duration::from_secs(2));
        }
    }
}

impl Default for MqttEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MqttEngine {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(std::time::Duration::from_secs(2));
        }
    }
}

async fn dispatcher(
    mut command_rx: UnboundedReceiver<MqttCommand>,
    connections: Arc<Mutex<HashMap<String, ConnectionHandle>>>,
    statuses: Arc<Mutex<HashMap<String, ConnectionStatus>>>,
    event_tx: UnboundedSender<MqttEvent>,
    max_connections: usize,
) {
    while let Some(command) = command_rx.recv().await {
        match command {
            MqttCommand::Connect(config) => {
                let id = config.id.clone();

                // Replace an existing connection with the same id.
                if let Some(previous) = connections.lock().ok().and_then(|mut c| c.remove(&id)) {
                    let _ = previous.cmd_tx.send(ConnCommand::Disconnect);
                }

                let active = connections.lock().map(|c| c.len()).unwrap_or(0);
                if active >= max_connections {
                    let _ = event_tx.send(MqttEvent::Error {
                        connection_id: Some(id),
                        message: format!(
                            "the maximum number of connections ({max_connections}) is already in use"
                        ),
                    });
                    continue;
                }

                let (cmd_tx, cmd_rx) = unbounded_channel();
                if let Ok(mut map) = connections.lock() {
                    map.insert(
                        id.clone(),
                        ConnectionHandle {
                            cmd_tx: cmd_tx.clone(),
                        },
                    );
                }
                if let Ok(mut map) = statuses.lock() {
                    map.insert(id.clone(), ConnectionStatus::Connecting);
                }

                session::spawn(config, cmd_rx, event_tx.clone(), statuses.clone());
            }
            MqttCommand::Disconnect(id) => {
                let handle = connections.lock().ok().and_then(|mut c| c.remove(&id));
                if let Some(handle) = handle {
                    let _ = handle.cmd_tx.send(ConnCommand::Disconnect);
                }
            }
            MqttCommand::Subscribe(request) => {
                if let Some(handle) = handle_for(&connections, &request.connection_id) {
                    let _ = handle.cmd_tx.send(ConnCommand::Subscribe {
                        topic: request.topic,
                        qos: request.qos,
                    });
                } else {
                    let _ = event_tx.send(MqttEvent::Error {
                        connection_id: Some(request.connection_id),
                        message: "the connection is not open".to_string(),
                    });
                }
            }
            MqttCommand::Unsubscribe {
                connection_id,
                topic,
            } => {
                if let Some(handle) = handle_for(&connections, &connection_id) {
                    let _ = handle.cmd_tx.send(ConnCommand::Unsubscribe { topic });
                }
            }
            MqttCommand::SetSubscriptionQos {
                connection_id,
                topic,
                qos,
            } => {
                if let Some(handle) = handle_for(&connections, &connection_id) {
                    // Resubscribing with a different QoS is how the QoS of an
                    // existing subscription is changed.
                    let _ = handle.cmd_tx.send(ConnCommand::Unsubscribe {
                        topic: topic.clone(),
                    });
                    let _ = handle.cmd_tx.send(ConnCommand::Subscribe { topic, qos });
                }
            }
            MqttCommand::Publish(request) => {
                if let Some(handle) = handle_for(&connections, &request.connection_id) {
                    let _ = handle.cmd_tx.send(ConnCommand::Publish(request));
                } else {
                    let _ = event_tx.send(MqttEvent::Error {
                        connection_id: Some(request.connection_id),
                        message: "the connection is not open".to_string(),
                    });
                }
            }
        }
    }

    // The command channel closed: tear every connection down.
    if let Ok(mut map) = connections.lock() {
        for (_, handle) in map.drain() {
            let _ = handle.cmd_tx.send(ConnCommand::Disconnect);
        }
    }
}

fn handle_for(
    connections: &Arc<Mutex<HashMap<String, ConnectionHandle>>>,
    connection_id: &str,
) -> Option<ConnectionHandle> {
    connections.lock().ok()?.get(connection_id).cloned()
}

impl Clone for ConnectionHandle {
    fn clone(&self) -> Self {
        Self {
            cmd_tx: self.cmd_tx.clone(),
        }
    }
}

/// Convenience: open a connection with the given configuration.
pub fn connect(engine: &MqttEngine, config: MqttConnection) -> Result<(), EngineError> {
    engine.execute(MqttCommand::Connect(config))
}
