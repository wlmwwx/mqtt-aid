//! SQLite persistence.
//!
//! V1.0 stores the *configuration* (connections, subscriptions, templates and
//! settings) in SQLite. Captured messages live in memory only; the `messages`
//! table exists so that a later version can persist them without a migration.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;

/// Errors from the storage layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("the database could not be serialised: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("the database file could not be opened: {0}")]
    Io(#[from] std::io::Error),
}

/// A handle to the application database.
pub struct Database {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    /// Open (and create if needed) the database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Database, StorageError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(&path)?;
        connection.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        let database = Database {
            connection: Mutex::new(connection),
            path,
        };
        database.migrate()?;
        Ok(database)
    }

    /// Open an in-memory database, used by the tests.
    pub fn open_in_memory() -> Result<Database, StorageError> {
        let connection = Connection::open_in_memory()?;
        let database = Database {
            connection: Mutex::new(connection),
            path: PathBuf::from(":memory:"),
        };
        database.migrate()?;
        Ok(database)
    }

    /// Where the database lives on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Run a statement against the database.
    pub fn execute(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<usize, StorageError> {
        let connection = self.connection.lock().expect("the database lock");
        Ok(connection.execute(sql, params)?)
    }

    /// Run a batch of statements.
    pub fn execute_batch(&self, sql: &str) -> Result<(), StorageError> {
        let connection = self.connection.lock().expect("the database lock");
        connection.execute_batch(sql)?;
        Ok(())
    }

    /// Query rows, mapping each row with `mapper`.
    pub fn query<T>(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
        mapper: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, StorageError> {
        let connection = self.connection.lock().expect("the database lock");
        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map(params, mapper)?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS connections (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                host        TEXT NOT NULL,
                port        INTEGER NOT NULL,
                data        TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS subscriptions (
                id            TEXT PRIMARY KEY,
                connection_id TEXT NOT NULL,
                topic         TEXT NOT NULL,
                qos           INTEGER NOT NULL,
                data          TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS publish_templates (
                id   TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS data_series (
                id            TEXT PRIMARY KEY,
                name          TEXT NOT NULL,
                connection_id TEXT NOT NULL,
                topic         TEXT NOT NULL,
                path          TEXT NOT NULL,
                data          TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id            TEXT PRIMARY KEY,
                connection_id TEXT NOT NULL,
                topic         TEXT NOT NULL,
                payload       BLOB NOT NULL,
                qos           INTEGER NOT NULL,
                retain        INTEGER NOT NULL,
                received_at   INTEGER NOT NULL,
                version       TEXT NOT NULL,
                properties    TEXT
            );
            "#,
        )
    }
}
