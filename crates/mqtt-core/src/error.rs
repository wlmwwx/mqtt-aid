use thiserror::Error;

/// Errors that can occur inside the pure domain layer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("invalid topic filter: {0}")]
    Topic(#[from] crate::topic::TopicError),
    #[error("payload format error: {0}")]
    Format(#[from] crate::format::FormatError),
}
