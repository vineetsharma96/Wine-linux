use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use androidpc_runtime_api::{Architecture, DisplayConfig, DisplayType};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required path: {0}")]
    MissingPath(String),

    #[error("Invalid memory limit '{0}MB': must be between 256MB and 32768MB")]
    InvalidMemoryLimit(u32),

    #[error("Invalid display resolution: {width}x{height}")]
    InvalidResolution { width: u32, height: u32 },

    #[error("Configuration parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Validation failed: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub base_data_dir: PathBuf,
    pub runtime_artifacts_dir: PathBuf,
    pub temporary_dir: PathBuf,
    pub debug_mode: bool,
    pub log_level: String,
    pub max_concurrent_processes: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            base_data_dir: PathBuf::from("/data/data/com.androidpc.environment/files"),
            runtime_artifacts_dir: PathBuf::from("/data/data/com.androidpc.environment/files/artifacts"),
            temporary_dir: PathBuf::from("/data/data/com.androidpc.environment/cache/tmp"),
            debug_mode: true,
            log_level: "info".to_string(),
            max_concurrent_processes: 64,
        }
    }
}

impl EngineConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.base_data_dir.as_os_str().is_empty() {
            return Err(ConfigError::MissingPath("base_data_dir".to_string()));
        }
        if self.runtime_artifacts_dir.as_os_str().is_empty() {
            return Err(ConfigError::MissingPath("runtime_artifacts_dir".to_string()));
        }
        if self.temporary_dir.as_os_str().is_empty() {
            return Err(ConfigError::MissingPath("temporary_dir".to_string()));
        }
        Ok(())
    }

    pub fn from_json(json_str: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_json::from_str(json_str)?;
        config.validate()?;
        Ok(config)
    }

    pub fn to_json(&self) -> Result<String, ConfigError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub id: String,
    pub distribution: String, // "ubuntu" | "debian"
    pub version: String,      // e.g. "24.04", "bookworm"
    pub architecture: Architecture,
    pub rootfs_directory: PathBuf,
    pub memory_limit_mb: u32,
    pub display: DisplayConfig,
    pub default_username: String,
    pub custom_mounts: Vec<(PathBuf, PathBuf)>, // host -> guest
}

impl EnvironmentConfig {
    pub fn new_default(
        id: impl Into<String>,
        distribution: impl Into<String>,
        base_dir: &Path,
    ) -> Self {
        let id_str = id.into();
        let distro_str = distribution.into();
        let rootfs = base_dir.join("environments").join(&id_str).join("rootfs");

        Self {
            id: id_str,
            distribution: distro_str,
            version: "latest".to_string(),
            architecture: Architecture::host(),
            rootfs_directory: rootfs,
            memory_limit_mb: 2048,
            display: DisplayConfig {
                display_type: DisplayType::Xvnc,
                width: 1280,
                height: 720,
                dpi: 160,
                display_number: 1,
                listen_address: "127.0.0.1".to_string(),
                port: 5901,
            },
            default_username: "user".to_string(),
            custom_mounts: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.id.trim().is_empty() {
            return Err(ConfigError::ValidationError("Environment ID cannot be empty".to_string()));
        }
        if self.memory_limit_mb < 256 || self.memory_limit_mb > 32768 {
            return Err(ConfigError::InvalidMemoryLimit(self.memory_limit_mb));
        }
        if self.display.width == 0 || self.display.height == 0 {
            return Err(ConfigError::InvalidResolution {
                width: self.display.width,
                height: self.display.height,
            });
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_config_default_and_validation() {
        let config = EngineConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.log_level, "info");
        assert_eq!(config.max_concurrent_processes, 64);
    }

    #[test]
    fn test_engine_config_json_roundtrip() {
        let config = EngineConfig::default();
        let json = config.to_json().expect("Serialization failed");
        let parsed = EngineConfig::from_json(&json).expect("Deserialization failed");
        assert_eq!(config.base_data_dir, parsed.base_data_dir);
    }

    #[test]
    fn test_environment_config_validation() {
        let env_config = EnvironmentConfig::new_default("test-env", "ubuntu", Path::new("/data/local"));
        assert!(env_config.validate().is_ok());
        assert_eq!(env_config.distribution, "ubuntu");

        let mut invalid_env = env_config.clone();
        invalid_env.memory_limit_mb = 50; // Below minimum 256MB
        assert!(invalid_env.validate().is_err());
    }
}
