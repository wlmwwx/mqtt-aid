use mqtt_core::models::{MqttConnection, PublishRequest, SubscribeRequest};

/// Commands sent from the GUI to the MQTT engine.
#[derive(Debug, Clone)]
pub enum MqttCommand {
    /// Open a connection to a broker.
    Connect(MqttConnection),
    /// Close a connection and stop its background task.
    Disconnect(String),
    /// Subscribe to a topic filter.
    Subscribe(SubscribeRequest),
    /// Remove a subscription.
    Unsubscribe {
        connection_id: String,
        topic: String,
    },
    /// Change the QoS of an existing subscription.
    SetSubscriptionQos {
        connection_id: String,
        topic: String,
        qos: u8,
    },
    /// Publish a message.
    Publish(PublishRequest),
}

/// Commands handled by a single connection task.
#[derive(Debug, Clone)]
pub enum ConnCommand {
    Subscribe { topic: String, qos: u8 },
    Unsubscribe { topic: String },
    Publish(PublishRequest),
    Disconnect,
}
