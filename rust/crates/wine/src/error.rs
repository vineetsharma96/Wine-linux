use thiserror::Error;

#[derive(Debug, Error)]
pub enum WineError {
    #[error("Wine prefix not found: {0}")]
    PrefixNotFound(String),

    #[error("Wine prefix already exists: {0}")]
    PrefixAlreadyExists(String),

    #[error("Windows binary (.exe/.msi) not found: {0}")]
    BinaryNotFound(String),

    #[error("Invalid Wine configuration: {0}")]
    InvalidConfig(String),

    #[error("Unsupported Windows binary format: {0}")]
    UnsupportedBinary(String),

    #[error("Supervisor error: {0}")]
    Supervisor(String),

    #[error("Security error: {0}")]
    Security(#[from] androidpc_security::SecurityError),

    #[error("Runtime error: {0}")]
    Runtime(#[from] androidpc_runtime_api::RuntimeError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
