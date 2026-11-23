//! Storage for the data series configurations.

use mqtt_core::models::DataSeries;

use crate::database::{Database, StorageError};

/// Saved data series.
pub struct DataSeriesRepo<'a> {
    database: &'a Database,
}

impl<'a> DataSeriesRepo<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Insert or update a series.
    pub fn save(&self, series: &DataSeries) -> Result<(), StorageError> {
        let data = serde_json::to_string(series)?;
        self.database.execute(
            "INSERT INTO data_series (id, name, connection_id, topic, path, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET name = ?2, connection_id = ?3, topic = ?4, path = ?5, data = ?6",
            &[
                &series.id,
                &series.name,
                &series.connection_id,
                &series.topic,
                &series.path,
                &data,
            ],
        )?;
        Ok(())
    }

    /// Every saved series.
    pub fn list(&self) -> Result<Vec<DataSeries>, StorageError> {
        let rows =
            self.database
                .query("SELECT data FROM data_series ORDER BY name", &[], |row| {
                    row.get::<_, String>(0)
                })?;
        let mut saved = Vec::with_capacity(rows.len());
        for row in rows {
            match serde_json::from_str::<DataSeries>(&row) {
                Ok(series) => saved.push(series),
                Err(error) => {
                    tracing::warn!("skipping a corrupted data series: {error}");
                }
            }
        }
        Ok(saved)
    }

    /// Remove a series.
    pub fn delete(&self, id: &str) -> Result<(), StorageError> {
        self.database
            .execute("DELETE FROM data_series WHERE id = ?1", &[&id])?;
        Ok(())
    }
}
