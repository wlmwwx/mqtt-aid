//! The MQTT Assistant application: the window, the state and the wiring.

pub mod app;
pub mod app_state;
#[cfg(feature = "screenshot")]
pub mod screenshot;

pub use app::MqttApp;
pub use app_state::AppState;
