//! Local persistence for MQTT Assistant.
//!
//! The storage layer only knows about *configuration*: connections,
//! subscriptions, publish templates and settings. Captured messages are kept
//! in memory by the application.

pub mod connection_repo;
pub mod data_series_repo;
pub mod database;
pub mod subscription_repo;
pub mod template_repo;

pub use connection_repo::ConnectionRepo;
pub use data_series_repo::DataSeriesRepo;
pub use database::{Database, StorageError};
pub use subscription_repo::SubscriptionRepo;
pub use template_repo::{SettingsRepo, TemplateRepo};
