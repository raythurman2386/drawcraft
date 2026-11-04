use thiserror::Error;

/// Errors returned while executing an editor command.
#[derive(Debug, Error)]
pub enum EditError {
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("missing required parameter: {0}")]
    MissingParam(&'static str),
    #[error("invalid parameter {name}: {reason}")]
    InvalidParam { name: &'static str, reason: String },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("document error: {0}")]
    Document(#[from] drawcraft_document::DocumentError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
