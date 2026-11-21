//! Storage for connection configurations.

use mqtt_core::models::MqttConnection;

use crate::database::{Database, StorageError};

/// Saved broker connections.
pub struct ConnectionRepo<'a> {
    database: &'a Database,
}

impl<'a> ConnectionRepo<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Insert or update a connection.
    pub fn save(&self, connection: &MqttConnection) -> Result<(), StorageError> {
        let data = serde_json::to_string(connection)?;
        self.database.execute(
            "INSERT INTO connections (id, name, host, port, data) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name = ?2, host = ?3, port = ?4, data = ?5",
            &[
                &connection.id,
                &connection.name,
                &connection.host,
                &connection.port,
                &data,
            ],
        )?;
        Ok(())
    }

    /// Every saved connection.
    pub fn list(&self) -> Result<Vec<MqttConnection>, StorageError> {
        let rows =
            self.database
                .query("SELECT data FROM connections ORDER BY name", &[], |row| {
                    row.get::<_, String>(0)
                })?;
        let mut connections = Vec::with_capacity(rows.len());
        for row in rows {
            match serde_json::from_str(&row) {
                Ok(connection) => connections.push(connection),
                Err(error) => {
                    tracing::warn!("skipping a corrupted connection: {error}");
                }
            }
        }
        Ok(connections)
    }

    /// A single connection by id.
    pub fn get(&self, id: &str) -> Result<Option<MqttConnection>, StorageError> {
        let rows = self.database.query(
            "SELECT data FROM connections WHERE id = ?1",
            &[&id],
            |row| row.get::<_, String>(0),
        )?;
        Ok(rows
            .first()
            .and_then(|data| serde_json::from_str(data).ok()))
    }

    /// Remove a connection.
    pub fn delete(&self, id: &str) -> Result<(), StorageError> {
        self.database
            .execute("DELETE FROM connections WHERE id = ?1", &[&id])?;
        Ok(())
    }
}
