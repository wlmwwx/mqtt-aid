use serde::{Deserialize, Serialize};

use crate::format::detect_format;

/// MQTT protocol dialect used for a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MqttVersion {
    /// MQTT 3.1.1
    #[default]
    V311,
    /// MQTT 5.0
    V5,
}

impl MqttVersion {
    pub fn label(self) -> &'static str {
        match self {
            MqttVersion::V311 => "3.1.1",
            MqttVersion::V5 => "5.0",
        }
    }
}

/// Lifecycle state of a single broker connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

impl ConnectionStatus {
    pub fn label(self) -> &'static str {
        match self {
            ConnectionStatus::Disconnected => "Disconnected",
            ConnectionStatus::Connecting => "Connecting",
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Reconnecting => "Reconnecting",
            ConnectionStatus::Error => "Error",
        }
    }

    /// Whether the connection currently carries traffic.
    pub fn is_active(self) -> bool {
        matches!(self, ConnectionStatus::Connected)
    }
}

/// How a payload should be presented to the user.
///
/// `Auto` is a *request* to detect the format; the actual rendering always
/// resolves it into one of the concrete variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PayloadFormat {
    #[default]
    Auto,
    Text,
    Hex,
    Json,
    Binary,
    Base64,
}

impl PayloadFormat {
    pub fn label(self) -> &'static str {
        match self {
            PayloadFormat::Auto => "Auto",
            PayloadFormat::Text => "Text",
            PayloadFormat::Hex => "Hex",
            PayloadFormat::Json => "JSON",
            PayloadFormat::Binary => "Binary",
            PayloadFormat::Base64 => "Base64",
        }
    }

    /// Every concrete format, i.e. `Auto` excluded.
    pub fn concrete() -> &'static [PayloadFormat] {
        &[
            PayloadFormat::Auto,
            PayloadFormat::Text,
            PayloadFormat::Hex,
            PayloadFormat::Json,
            PayloadFormat::Binary,
            PayloadFormat::Base64,
        ]
    }

    /// Resolve `Auto` into a concrete format for the given bytes.
    pub fn resolve(self, data: &[u8]) -> PayloadFormat {
        match self {
            PayloadFormat::Auto => detect_format(data),
            other => other,
        }
    }
}

/// A saved broker connection.
///
/// The password is *not* part of this struct: it lives in the OS keyring and is
/// referenced through [`MqttConnection::credential_ref`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MqttConnection {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub protocol: MqttVersion,
    pub client_id: String,
    pub username: Option<String>,
    /// Keyring account under which the password is stored.
    pub credential_ref: Option<String>,
    pub keep_alive: u64,
    pub clean_session: bool,
    pub auto_reconnect: bool,
    pub tls: bool,
    pub ca_certificate: Option<String>,
    pub client_certificate: Option<String>,
    pub client_key: Option<String>,
}

impl MqttConnection {
    /// A connection with sane defaults for a plain TCP broker.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        host: impl Into<String>,
        port: u16,
    ) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            name: name.into(),
            host: host.into(),
            port,
            protocol: MqttVersion::V311,
            client_id: format!("mqtt-assistant-{id}"),
            username: None,
            credential_ref: None,
            keep_alive: 60,
            clean_session: true,
            auto_reconnect: true,
            tls: false,
            ca_certificate: None,
            client_certificate: None,
            client_key: None,
        }
    }
}

/// A topic subscription belonging to one connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MqttSubscription {
    pub id: String,
    pub connection_id: String,
    pub topic: String,
    pub qos: u8,
    /// Human friendly name shown instead of the raw topic.
    pub alias: Option<String>,
    /// Default rendering format for messages of this subscription.
    pub format: PayloadFormat,
    /// When true the messages still arrive but are not displayed.
    pub paused: bool,
}

impl MqttSubscription {
    pub fn new(
        id: impl Into<String>,
        connection_id: impl Into<String>,
        topic: impl Into<String>,
        qos: u8,
    ) -> Self {
        Self {
            id: id.into(),
            connection_id: connection_id.into(),
            topic: topic.into(),
            qos,
            alias: None,
            format: PayloadFormat::Auto,
            paused: false,
        }
    }

    /// Name shown in the UI: the alias when present, otherwise the topic.
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.topic)
    }
}

/// MQTT 5.0 publish properties, in a form that does not depend on the wire
/// library.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MqttProperties {
    pub payload_format_indicator: Option<u8>,
    pub message_expiry_interval: Option<u32>,
    pub response_topic: Option<String>,
    pub correlation_data: Option<Vec<u8>>,
    pub user_properties: Vec<(String, String)>,
    pub content_type: Option<String>,
}

impl MqttProperties {
    pub fn is_empty(&self) -> bool {
        self.payload_format_indicator.is_none()
            && self.message_expiry_interval.is_none()
            && self.response_topic.is_none()
            && self.correlation_data.is_none()
            && self.user_properties.is_empty()
            && self.content_type.is_none()
    }
}

/// A received MQTT message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MqttMessage {
    pub id: String,
    pub connection_id: String,
    pub topic: String,
    /// The payload exactly as it came off the wire.
    pub payload: Vec<u8>,
    pub qos: u8,
    pub retain: bool,
    /// Unix epoch milliseconds.
    pub received_at: i64,
    pub version: MqttVersion,
    pub properties: Option<MqttProperties>,
}

/// Request to publish a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishRequest {
    pub connection_id: String,
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: u8,
    pub retain: bool,
    pub properties: Option<MqttProperties>,
}

/// Request to subscribe to a topic filter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub connection_id: String,
    pub id: String,
    pub topic: String,
    pub qos: u8,
}

/// A saved publish template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishTemplate {
    pub id: String,
    pub name: String,
    pub topic: String,
    pub payload: Vec<u8>,
    pub format: PayloadFormat,
    pub qos: u8,
    pub retain: bool,
}
