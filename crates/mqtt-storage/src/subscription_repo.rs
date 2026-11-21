//! Storage for subscription configurations.

use mqtt_core::models::MqttSubscription;

use crate::database::{Database, StorageError};

/// Saved topic subscriptions.
pub struct SubscriptionRepo<'a> {
    database: &'a Database,
}

impl<'a> SubscriptionRepo<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Insert or update a subscription.
    pub fn save(&self, subscription: &MqttSubscription) -> Result<(), StorageError> {
        let data = serde_json::to_string(subscription)?;
        self.database.execute(
            "INSERT INTO subscriptions (id, connection_id, topic, qos, data)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET connection_id = ?2, topic = ?3, qos = ?4, data = ?5",
            &[
                &subscription.id,
                &subscription.connection_id,
                &subscription.topic,
                &subscription.qos,
                &data,
            ],
        )?;
        Ok(())
    }

    /// Every saved subscription.
    pub fn list(&self) -> Result<Vec<MqttSubscription>, StorageError> {
        let rows = self.database.query(
            "SELECT data FROM subscriptions ORDER BY topic",
            &[],
            |row| row.get::<_, String>(0),
        )?;
        let mut subscriptions = Vec::with_capacity(rows.len());
        for row in rows {
            match serde_json::from_str(&row) {
                Ok(subscription) => subscriptions.push(subscription),
                Err(error) => {
                    tracing::warn!("skipping a corrupted subscription: {error}");
                }
            }
        }
        Ok(subscriptions)
    }

    /// The subscriptions of one connection.
    pub fn list_for_connection(
        &self,
        connection_id: &str,
    ) -> Result<Vec<MqttSubscription>, StorageError> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|subscription| subscription.connection_id == connection_id)
            .collect())
    }

    /// Remove a subscription.
    pub fn delete(&self, id: &str) -> Result<(), StorageError> {
        self.database
            .execute("DELETE FROM subscriptions WHERE id = ?1", &[&id])?;
        Ok(())
    }
}
