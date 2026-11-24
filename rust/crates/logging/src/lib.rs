use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Error)]
pub enum LogError {
    #[error("Logging subsystem already initialized")]
    AlreadyInitialized,
    #[error("Failed to parse log filter level: {0}")]
    InvalidFilter(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp_epoch_ms: u64,
    pub level: String,
    pub target: String,
    pub message: String,
}

static LOG_BUFFER: OnceLock<Arc<Mutex<VecDeque<LogEntry>>>> = OnceLock::new();

fn get_or_init_buffer() -> Arc<Mutex<VecDeque<LogEntry>>> {
    LOG_BUFFER.get_or_init(|| Arc::new(Mutex::new(VecDeque::with_capacity(1000)))).clone()
}

pub fn push_log(level: &str, target: &str, message: &str) {
    let buffer = get_or_init_buffer();
    let mut guard = match buffer.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    if guard.len() >= 1000 {
        guard.pop_front();
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    guard.push_back(LogEntry {
        timestamp_epoch_ms: timestamp,
        level: level.to_string(),
        target: target.to_string(),
        message: message.to_string(),
    });
}

pub fn get_recent_logs(limit: usize) -> Vec<LogEntry> {
    let buffer = get_or_init_buffer();
    let guard = match buffer.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    let start = if guard.len() > limit {
        guard.len() - limit
    } else {
        0
    };

    guard.iter().skip(start).cloned().collect()
}

pub fn init_logging(log_level: &str) -> Result<(), LogError> {
    let filter = EnvFilter::try_new(log_level)
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|e| LogError::InvalidFilter(e.to_string()))?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init();

    push_log("INFO", "androidpc::logging", "Logging initialized");
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer_push_and_retrieval() {
        let _ = init_logging("debug");
        push_log("INFO", "test::target", "Test message 1");
        push_log("WARN", "test::target", "Test message 2");

        let logs = get_recent_logs(10);
        assert!(!logs.is_empty());
        let last = logs.last().expect("Logs should not be empty");
        assert_eq!(last.message, "Test message 2");
        assert_eq!(last.level, "WARN");
    }
}
