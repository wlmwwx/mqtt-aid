//! Core domain model for MQTT Assistant.
//!
//! This crate is deliberately free of any GUI, async runtime or network
//! dependency: it holds the vocabulary (models), the payload format engine and
//! the MQTT topic rules that the rest of the application is built on top of.

pub mod error;
pub mod extract;
pub mod format;
pub mod models;
pub mod topic;

pub use error::CoreError;
pub use extract::extract_value;
pub use format::{
    Base64Codec, BinaryCodec, FormatError, HexCodec, JsonCodec, PayloadCodec, TextCodec,
    detect_format, format_json, hex_dump, minify_json, parse_json,
};
pub use models::{
    ConnectionStatus, DataPoint, DataSeries, MqttConnection, MqttMessage, MqttProperties,
    MqttSubscription, MqttVersion, PayloadFormat, PublishRequest, PublishTemplate,
    SubscribeRequest,
};
pub use topic::{TopicError, is_valid_publish_topic, matches, validate_filter};
