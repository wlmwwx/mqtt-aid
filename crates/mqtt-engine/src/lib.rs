//! The MQTT engine: background connections, commands in, events out.
//!
//! The engine owns a Tokio runtime that lives entirely outside the GUI thread.
//! The GUI talks to it through two channels: [`MqttCommand`]s go in,
//! [`MqttEvent`]s come out.

pub mod command;
pub mod credentials;
pub mod engine;
pub mod event;
pub mod session;

pub use command::{ConnCommand, MqttCommand};
pub use engine::{DEFAULT_MAX_CONNECTIONS, EngineError, MqttEngine, connect};
pub use event::MqttEvent;
