//! Storage for publish templates and user settings.

use mqtt_core::models::PublishTemplate;

use crate::database::{Database, StorageError};

/// Saved publish templates.
pub struct TemplateRepo<'a> {
    database: &'a Database,
}

impl<'a> TemplateRepo<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Insert or update a template.
    pub fn save(&self, template: &PublishTemplate) -> Result<(), StorageError> {
        let data = serde_json::to_string(template)?;
        self.database.execute(
            "INSERT INTO publish_templates (id, name, data) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET name = ?2, data = ?3",
            &[&template.id, &template.name, &data],
        )?;
        Ok(())
    }

    /// Every saved template.
    pub fn list(&self) -> Result<Vec<PublishTemplate>, StorageError> {
        let rows = self.database.query(
            "SELECT data FROM publish_templates ORDER BY name",
            &[],
            |row| row.get::<_, String>(0),
        )?;
        let mut templates = Vec::with_capacity(rows.len());
        for row in rows {
            match serde_json::from_str(&row) {
                Ok(template) => templates.push(template),
                Err(error) => {
                    tracing::warn!("skipping a corrupted template: {error}");
                }
            }
        }
        Ok(templates)
    }

    /// Remove a template.
    pub fn delete(&self, id: &str) -> Result<(), StorageError> {
        self.database
            .execute("DELETE FROM publish_templates WHERE id = ?1", &[&id])?;
        Ok(())
    }
}

/// User settings, stored as key/value pairs.
pub struct SettingsRepo<'a> {
    database: &'a Database,
}

impl<'a> SettingsRepo<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Store a setting.
    pub fn set(&self, key: &str, value: &str) -> Result<(), StorageError> {
        self.database.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            &[&key, &value],
        )?;
        Ok(())
    }

    /// Read a setting.
    pub fn get(&self, key: &str) -> Result<Option<String>, StorageError> {
        let rows = self.database.query(
            "SELECT value FROM app_settings WHERE key = ?1",
            &[&key],
            |row| row.get::<_, String>(0),
        )?;
        Ok(rows.into_iter().next())
    }

    /// Remove a setting.
    pub fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.database
            .execute("DELETE FROM app_settings WHERE key = ?1", &[&key])?;
        Ok(())
    }
}
