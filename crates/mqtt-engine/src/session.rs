//! Per-connection background tasks.
//!
//! Every broker connection owns one task that drives a `rumqttc` event loop.
//! The task multiplexes two sources: commands coming from the GUI and events
//! coming from the network. It never blocks the GUI thread and never blocks on
//! the network while a command is waiting.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mqtt_core::models::{
    ConnectionStatus, MqttConnection, MqttMessage, MqttProperties, MqttVersion, PublishRequest,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::command::ConnCommand;
use crate::credentials::resolve_password;
use crate::event::MqttEvent;

/// Largest MQTT packet we accept or send, in bytes.
const MAX_PACKET_SIZE: usize = 16 * 1024 * 1024;
/// Capacity of the rumqttc request queue.
const REQUEST_CHANNEL_CAPACITY: usize = 1024;
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Shared state a session needs to talk to the rest of the engine.
#[derive(Clone)]
struct Context {
    id: String,
    event_tx: UnboundedSender<MqttEvent>,
    statuses: Arc<Mutex<HashMap<String, ConnectionStatus>>>,
}

impl Context {
    fn new(
        id: String,
        event_tx: UnboundedSender<MqttEvent>,
        statuses: Arc<Mutex<HashMap<String, ConnectionStatus>>>,
    ) -> Self {
        Self {
            id,
            event_tx,
            statuses,
        }
    }

    fn emit(&self, event: MqttEvent) {
        let _ = self.event_tx.send(event);
    }

    fn set_status(&self, status: ConnectionStatus) {
        if let Ok(mut statuses) = self.statuses.lock() {
            statuses.insert(self.id.clone(), status);
        }
        self.emit(MqttEvent::ConnectionStatusChanged {
            connection_id: self.id.clone(),
            status,
        });
    }

    fn fail(&self, message: String) {
        self.emit(MqttEvent::Error {
            connection_id: Some(self.id.clone()),
            message,
        });
    }
}

/// Spawn the background task for a connection.
pub fn spawn(
    config: MqttConnection,
    cmd_rx: UnboundedReceiver<ConnCommand>,
    event_tx: UnboundedSender<MqttEvent>,
    statuses: Arc<Mutex<HashMap<String, ConnectionStatus>>>,
) {
    tokio::spawn(async move {
        match config.protocol {
            MqttVersion::V311 => run_v311(config, cmd_rx, event_tx, statuses).await,
            MqttVersion::V5 => run_v5(config, cmd_rx, event_tx, statuses).await,
        }
    });
}

/// Track which topics a connection is subscribed to, so that they can be
/// restored after a reconnect.
#[derive(Default)]
struct Subscriptions {
    entries: Vec<(String, u8)>,
}

impl Subscriptions {
    fn note(&mut self, op: &ConnCommand) {
        match op {
            ConnCommand::Subscribe { topic, qos } => {
                if let Some(existing) = self.entries.iter_mut().find(|(t, _)| t == topic) {
                    existing.1 = *qos;
                } else {
                    self.entries.push((topic.clone(), *qos));
                }
            }
            ConnCommand::Unsubscribe { topic } => {
                self.entries.retain(|(t, _)| t != topic);
            }
            _ => {}
        }
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

fn next_message_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("msg-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn v4_qos(level: u8) -> rumqttc::QoS {
    match level {
        1 => rumqttc::QoS::AtLeastOnce,
        2 => rumqttc::QoS::ExactlyOnce,
        _ => rumqttc::QoS::AtMostOnce,
    }
}

fn v5_qos(level: u8) -> rumqttc::v5::mqttbytes::QoS {
    match level {
        1 => rumqttc::v5::mqttbytes::QoS::AtLeastOnce,
        2 => rumqttc::v5::mqttbytes::QoS::ExactlyOnce,
        _ => rumqttc::v5::mqttbytes::QoS::AtMostOnce,
    }
}

fn qos_level(qos: rumqttc::QoS) -> u8 {
    match qos {
        rumqttc::QoS::AtMostOnce => 0,
        rumqttc::QoS::AtLeastOnce => 1,
        rumqttc::QoS::ExactlyOnce => 2,
    }
}

/// Build the TLS transport for a connection.
///
/// A CA certificate is mandatory for client certificate authentication: the
/// `Simple` TLS configuration only accepts an explicit CA chain.
fn tls_transport(config: &MqttConnection) -> Result<Option<rumqttc::Transport>, String> {
    if !config.tls {
        return Ok(None);
    }

    let client_auth = match (&config.client_certificate, &config.client_key) {
        (Some(cert), Some(key)) => {
            let cert =
                std::fs::read(cert).map_err(|e| format!("cannot read client certificate: {e}"))?;
            let key = std::fs::read(key).map_err(|e| format!("cannot read client key: {e}"))?;
            Some((cert, key))
        }
        (None, None) => None,
        _ => return Err("client certificate and client key must be set together".to_string()),
    };

    match &config.ca_certificate {
        Some(path) => {
            let ca = std::fs::read(path).map_err(|e| format!("cannot read CA certificate: {e}"))?;
            Ok(Some(rumqttc::Transport::tls(ca, client_auth, None)))
        }
        None if client_auth.is_some() => Err(
            "client certificate authentication requires a CA certificate to be configured"
                .to_string(),
        ),
        None => Ok(Some(rumqttc::Transport::tls_with_config(
            rumqttc::TlsConfiguration::default(),
        ))),
    }
}

/// MQTT 3.1.1 session.
pub async fn run_v311(
    config: MqttConnection,
    mut cmd_rx: UnboundedReceiver<ConnCommand>,
    event_tx: UnboundedSender<MqttEvent>,
    statuses: Arc<Mutex<HashMap<String, ConnectionStatus>>>,
) {
    let ctx = Context::new(config.id.clone(), event_tx, statuses);

    let password = resolve_password(&config).await;
    let mut options =
        rumqttc::MqttOptions::new(config.client_id.clone(), config.host.clone(), config.port);
    options.set_keep_alive(Duration::from_secs(config.keep_alive.max(1)));
    options.set_clean_session(config.clean_session);
    options.set_max_packet_size(MAX_PACKET_SIZE, MAX_PACKET_SIZE);
    options.set_request_channel_capacity(REQUEST_CHANNEL_CAPACITY);

    if let Some(username) = &config.username {
        options.set_credentials(username.clone(), password.clone().unwrap_or_default());
    }

    match tls_transport(&config) {
        Ok(Some(transport)) => {
            options.set_transport(transport);
        }
        Ok(None) => {}
        Err(message) => {
            ctx.set_status(ConnectionStatus::Error);
            ctx.fail(message);
            return;
        }
    }

    let (client, mut eventloop) = rumqttc::AsyncClient::new(options, 10);
    let mut subscriptions = Subscriptions::default();
    let mut pending: VecDeque<ConnCommand> = VecDeque::new();
    let mut pending_subscriptions: VecDeque<(String, u8)> = VecDeque::new();
    let mut backoff = INITIAL_BACKOFF;
    let mut failed = false;

    ctx.set_status(ConnectionStatus::Connecting);

    loop {
        if let Some(op) = pending.pop_front_if(|op| apply_v311(&client, op).is_ok()) {
            subscriptions.note(&op);
            note_subscription(&mut pending_subscriptions, &op);
            report(&ctx, &op);
        }

        tokio::select! {
            biased;
            command = cmd_rx.recv() => {
                match command {
                    None => break,
                    Some(ConnCommand::Disconnect) => {
                        let _ = client.try_disconnect();
                        break;
                    }
                    Some(op) => {
                        match apply_v311(&client, &op) {
                            Ok(()) => {
                                subscriptions.note(&op);
                                note_subscription(&mut pending_subscriptions, &op);
                                report(&ctx, &op);
                            }
                            Err(_) => pending.push_back(op),
                        }
                    }
                }
            }
            event = eventloop.poll() => {
                match event {
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(ack))) => {
                        if ack.code == rumqttc::ConnectReturnCode::Success {
                            ctx.set_status(ConnectionStatus::Connected);
                            backoff = INITIAL_BACKOFF;
                            for (topic, qos) in &subscriptions.entries {
                                let _ = client.try_subscribe(topic.clone(), v4_qos(*qos));
                                pending_subscriptions.push_back((topic.clone(), *qos));
                            }
                        } else {
                            ctx.set_status(ConnectionStatus::Error);
                            ctx.fail(format!("broker refused the connection: {:?}", ack.code));
                            if !config.auto_reconnect {
                                failed = true;
                                break;
                            }
                        }
                    }
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_))) => {
                        acknowledge_subscriptions(&ctx, &mut pending_subscriptions);
                    }
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                        let message = MqttMessage {
                            id: next_message_id(),
                            connection_id: ctx.id.clone(),
                            topic: publish.topic.clone(),
                            payload: publish.payload.to_vec(),
                            qos: qos_level(publish.qos),
                            retain: publish.retain,
                            received_at: now_millis(),
                            version: MqttVersion::V311,
                            properties: None,
                        };
                        ctx.emit(MqttEvent::MessageReceived(message));
                    }
                    Ok(_) => {}
                    Err(error) => {
                        if config.auto_reconnect {
                            ctx.set_status(ConnectionStatus::Reconnecting);
                            tokio::time::sleep(backoff).await;
                            backoff = (backoff * 2).min(MAX_BACKOFF);
                        } else {
                            ctx.set_status(ConnectionStatus::Error);
                            ctx.fail(format!("connection lost: {error}"));
                            failed = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    if !failed {
        ctx.set_status(ConnectionStatus::Disconnected);
    }
}

fn apply_v311(client: &rumqttc::AsyncClient, op: &ConnCommand) -> Result<(), String> {
    match op {
        ConnCommand::Subscribe { topic, qos } => client
            .try_subscribe(topic.clone(), v4_qos(*qos))
            .map_err(|error| error.to_string()),
        ConnCommand::Unsubscribe { topic } => client
            .try_unsubscribe(topic.clone())
            .map_err(|error| error.to_string()),
        ConnCommand::Publish(request) => publish_v311(client, request),
        ConnCommand::Disconnect => Ok(()),
    }
}

fn publish_v311(client: &rumqttc::AsyncClient, request: &PublishRequest) -> Result<(), String> {
    mqtt_core::topic::is_valid_publish_topic(&request.topic).map_err(|error| error.to_string())?;
    client
        .try_publish(
            request.topic.clone(),
            v4_qos(request.qos),
            request.retain,
            request.payload.clone(),
        )
        .map_err(|error| error.to_string())
}

/// MQTT 5.0 session.
pub async fn run_v5(
    config: MqttConnection,
    mut cmd_rx: UnboundedReceiver<ConnCommand>,
    event_tx: UnboundedSender<MqttEvent>,
    statuses: Arc<Mutex<HashMap<String, ConnectionStatus>>>,
) {
    let ctx = Context::new(config.id.clone(), event_tx, statuses);

    let password = resolve_password(&config).await;
    let mut options =
        rumqttc::v5::MqttOptions::new(config.client_id.clone(), config.host.clone(), config.port);
    options.set_keep_alive(Duration::from_secs(config.keep_alive.max(1)));
    options.set_clean_start(config.clean_session);
    options.set_max_packet_size(Some(MAX_PACKET_SIZE as u32));
    options.set_request_channel_capacity(REQUEST_CHANNEL_CAPACITY);

    if let Some(username) = &config.username {
        options.set_credentials(username.clone(), password.clone().unwrap_or_default());
    }

    match tls_transport(&config) {
        Ok(Some(transport)) => {
            options.set_transport(transport);
        }
        Ok(None) => {}
        Err(message) => {
            ctx.set_status(ConnectionStatus::Error);
            ctx.fail(message);
            return;
        }
    }

    let (client, mut eventloop) = rumqttc::v5::AsyncClient::new(options, 10);
    let mut subscriptions = Subscriptions::default();
    let mut pending: VecDeque<ConnCommand> = VecDeque::new();
    let mut pending_subscriptions: VecDeque<(String, u8)> = VecDeque::new();
    let mut backoff = INITIAL_BACKOFF;
    let mut failed = false;

    ctx.set_status(ConnectionStatus::Connecting);

    loop {
        if let Some(op) = pending.pop_front_if(|op| apply_v5(&client, op).is_ok()) {
            subscriptions.note(&op);
            note_subscription(&mut pending_subscriptions, &op);
            report(&ctx, &op);
        }

        tokio::select! {
            biased;
            command = cmd_rx.recv() => {
                match command {
                    None => break,
                    Some(ConnCommand::Disconnect) => {
                        let _ = client.try_disconnect();
                        break;
                    }
                    Some(op) => {
                        match apply_v5(&client, &op) {
                            Ok(()) => {
                                subscriptions.note(&op);
                                note_subscription(&mut pending_subscriptions, &op);
                                report(&ctx, &op);
                            }
                            Err(_) => pending.push_back(op),
                        }
                    }
                }
            }
            event = eventloop.poll() => {
                match event {
                    Ok(rumqttc::v5::Event::Incoming(rumqttc::v5::mqttbytes::v5::Packet::ConnAck(ack))) => {
                        if ack.code == rumqttc::v5::mqttbytes::v5::ConnectReturnCode::Success {
                            ctx.set_status(ConnectionStatus::Connected);
                            backoff = INITIAL_BACKOFF;
                            for (topic, qos) in &subscriptions.entries {
                                let _ = client.try_subscribe(topic.clone(), v5_qos(*qos));
                                pending_subscriptions.push_back((topic.clone(), *qos));
                            }
                        } else {
                            ctx.set_status(ConnectionStatus::Error);
                            ctx.fail(format!("broker refused the connection: {:?}", ack.code));
                            if !config.auto_reconnect {
                                failed = true;
                                break;
                            }
                        }
                    }
                    Ok(rumqttc::v5::Event::Incoming(rumqttc::v5::mqttbytes::v5::Packet::SubAck(_))) => {
                        acknowledge_subscriptions(&ctx, &mut pending_subscriptions);
                    }
                    Ok(rumqttc::v5::Event::Incoming(rumqttc::v5::mqttbytes::v5::Packet::Publish(publish))) => {
                        let message = MqttMessage {
                            id: next_message_id(),
                            connection_id: ctx.id.clone(),
                            topic: String::from_utf8_lossy(&publish.topic).to_string(),
                            payload: publish.payload.to_vec(),
                            qos: match publish.qos {
                                rumqttc::v5::mqttbytes::QoS::AtMostOnce => 0,
                                rumqttc::v5::mqttbytes::QoS::AtLeastOnce => 1,
                                rumqttc::v5::mqttbytes::QoS::ExactlyOnce => 2,
                            },
                            retain: publish.retain,
                            received_at: now_millis(),
                            version: MqttVersion::V5,
                            properties: publish.properties.as_ref().map(properties_from_v5),
                        };
                        ctx.emit(MqttEvent::MessageReceived(message));
                    }
                    Ok(_) => {}
                    Err(error) => {
                        if config.auto_reconnect {
                            ctx.set_status(ConnectionStatus::Reconnecting);
                            tokio::time::sleep(backoff).await;
                            backoff = (backoff * 2).min(MAX_BACKOFF);
                        } else {
                            ctx.set_status(ConnectionStatus::Error);
                            ctx.fail(format!("connection lost: {error}"));
                            failed = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    if !failed {
        ctx.set_status(ConnectionStatus::Disconnected);
    }
}

fn apply_v5(client: &rumqttc::v5::AsyncClient, op: &ConnCommand) -> Result<(), String> {
    match op {
        ConnCommand::Subscribe { topic, qos } => client
            .try_subscribe(topic.clone(), v5_qos(*qos))
            .map_err(|error| error.to_string()),
        ConnCommand::Unsubscribe { topic } => client
            .try_unsubscribe(topic.clone())
            .map_err(|error| error.to_string()),
        ConnCommand::Publish(request) => publish_v5(client, request),
        ConnCommand::Disconnect => Ok(()),
    }
}

fn publish_v5(client: &rumqttc::v5::AsyncClient, request: &PublishRequest) -> Result<(), String> {
    mqtt_core::topic::is_valid_publish_topic(&request.topic).map_err(|error| error.to_string())?;
    match &request.properties {
        Some(properties) => client
            .try_publish_with_properties(
                request.topic.clone(),
                v5_qos(request.qos),
                request.retain,
                request.payload.clone(),
                properties_to_v5(properties),
            )
            .map_err(|error| error.to_string()),
        None => client
            .try_publish(
                request.topic.clone(),
                v5_qos(request.qos),
                request.retain,
                request.payload.clone(),
            )
            .map_err(|error| error.to_string()),
    }
}

fn properties_to_v5(properties: &MqttProperties) -> rumqttc::v5::mqttbytes::v5::PublishProperties {
    rumqttc::v5::mqttbytes::v5::PublishProperties {
        payload_format_indicator: properties.payload_format_indicator,
        message_expiry_interval: properties.message_expiry_interval,
        topic_alias: None,
        response_topic: properties.response_topic.clone(),
        correlation_data: properties.correlation_data.clone().map(Into::into),
        user_properties: properties.user_properties.clone(),
        subscription_identifiers: Vec::new(),
        content_type: properties.content_type.clone(),
    }
}

fn properties_from_v5(
    properties: &rumqttc::v5::mqttbytes::v5::PublishProperties,
) -> MqttProperties {
    MqttProperties {
        payload_format_indicator: properties.payload_format_indicator,
        message_expiry_interval: properties.message_expiry_interval,
        response_topic: properties.response_topic.clone(),
        correlation_data: properties
            .correlation_data
            .as_ref()
            .map(|data| data.to_vec()),
        user_properties: properties.user_properties.clone(),
        content_type: properties.content_type.clone(),
    }
}

/// Tell the GUI that a command was accepted by the client.
fn report(ctx: &Context, op: &ConnCommand) {
    if let ConnCommand::Publish(request) = op {
        ctx.emit(MqttEvent::PublishCompleted {
            connection_id: ctx.id.clone(),
            topic: request.topic.clone(),
        })
    }
}

/// Remember a subscribe request until the broker acknowledges it.
fn note_subscription(queue: &mut VecDeque<(String, u8)>, op: &ConnCommand) {
    if let ConnCommand::Subscribe { topic, qos } = op {
        queue.push_back((topic.clone(), *qos));
    }
}

/// Report the subscriptions the broker just acknowledged.
fn acknowledge_subscriptions(ctx: &Context, queue: &mut VecDeque<(String, u8)>) {
    while let Some((topic, qos)) = queue.pop_front() {
        ctx.emit(MqttEvent::SubscriptionAcknowledged {
            connection_id: ctx.id.clone(),
            topic,
            qos,
        });
    }
}
