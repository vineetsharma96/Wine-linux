use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Environment initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Distribution '{0}' is not supported on architecture '{1}'")]
    UnsupportedArchitecture(String, String),

    #[error("Rootfs path not found: {0}")]
    RootfsNotFound(String),

    #[error("Integrity verification failed for '{0}': expected SHA-256 {1}, computed {2}")]
    ChecksumMismatch(String, String, String),

    #[error("Process supervisor error: {0}")]
    SupervisorError(String),

    #[error("Display backend error: {0}")]
    DisplayError(String),

    #[error("Terminal session error: {0}")]
    TerminalError(String),

    #[error("Operation timed out: {0}")]
    Timeout(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("System IO failure: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization / format error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal engine error: {0}")]
    Internal(String),
}
