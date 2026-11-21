//! Password storage in the operating system keyring.
//!
//! The password for a connection never lives in the connection configuration:
//! only a reference (the connection id) is stored, and the secret itself is
//! kept in the keychain.

use mqtt_core::models::MqttConnection;

/// Keyring service name used for all MQTT Assistant credentials.
pub const SERVICE: &str = "mqtt-assistant";

/// Store a password for a connection.
pub fn store_password(connection_id: &str, password: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, connection_id).map_err(|error| error.to_string())?;
    entry
        .set_password(password)
        .map_err(|error| error.to_string())
}

/// Read the password for a connection, if one is stored.
pub fn load_password(connection_id: &str) -> Option<String> {
    let entry = keyring::Entry::new(SERVICE, connection_id).ok()?;
    entry.get_password().ok()
}

/// Remove the password for a connection.
pub fn delete_password(connection_id: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE, connection_id).map_err(|error| error.to_string())?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// Resolve the password for a connection, following its credential reference.
pub async fn resolve_password(config: &MqttConnection) -> Option<String> {
    let reference = config.credential_ref.clone()?;
    tokio::task::spawn_blocking(move || load_password(&reference))
        .await
        .ok()
        .flatten()
}
