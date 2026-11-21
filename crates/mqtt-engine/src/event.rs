use mqtt_core::models::{ConnectionStatus, MqttMessage};

/// Events pushed from the MQTT engine to the GUI.
#[derive(Debug, Clone)]
pub enum MqttEvent {
    /// The lifecycle state of a connection changed.
    ConnectionStatusChanged {
        connection_id: String,
        status: ConnectionStatus,
    },
    /// A message arrived.
    MessageReceived(MqttMessage),
    /// A publish was handed over to the broker.
    PublishCompleted {
        connection_id: String,
        topic: String,
    },
    /// The broker acknowledged a subscription.
    SubscriptionAcknowledged {
        connection_id: String,
        topic: String,
        qos: u8,
    },
    /// Something went wrong.
    Error {
        connection_id: Option<String>,
        message: String,
    },
}
