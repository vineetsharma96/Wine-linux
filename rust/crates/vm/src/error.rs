use androidpc_runtime_api::RuntimeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),

    #[error("Invalid VM configuration: {0}")]
    InvalidConfig(String),

    #[error("VM not found: {0}")]
    NotFound(String),

    #[error("VM already exists: {0}")]
    AlreadyExists(String),

    #[error("VM '{0}' is already running")]
    AlreadyRunning(String),

    #[error("VM '{0}' is not running")]
    NotRunning(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Snapshot error: {0}")]
    Snapshot(String),

    #[error("QMP monitor error: {0}")]
    Monitor(String),

    #[error("Security validation failed: {0}")]
    Security(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
