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
    #[serde(default)]
    pub native_lib_dir: Option<PathBuf>,
    #[serde(default)]
    pub trusted_rootfs_key_hex: Option<String>,
    #[serde(default)]
    pub app_version_code: u32,
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
            native_lib_dir: None,
            trusted_rootfs_key_hex: None,
            app_version_code: 0,
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
    #[serde(default = "environment_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub created_at_epoch_ms: Option<u64>,
    #[serde(default)]
    pub verified_at_epoch_ms: Option<u64>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub last_started_at_epoch_ms: Option<u64>,
    #[serde(default)]
    pub runtime_version: Option<String>,
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

fn environment_schema_version() -> u32 {
    1
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
            schema_version: environment_schema_version(),
            created_at_epoch_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|time| time.as_millis() as u64),
            verified_at_epoch_ms: None,
            display_name: None,
            last_started_at_epoch_ms: None,
            runtime_version: None,
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
        Self::validate_id(&self.id)?;
        Self::validate_id(&self.default_username)?;
        if self.display_name.as_ref().is_some_and(|name| {
            name.trim().is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control)
        }) {
            return Err(ConfigError::ValidationError("Use a name with 1–80 characters and no control characters".into()));
        }
        if self.schema_version != environment_schema_version() {
            return Err(ConfigError::ValidationError("Unsupported environment configuration schema".into()));
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

    pub fn validate_id(id: &str) -> Result<(), ConfigError> {
        if id.is_empty() || id.len() > 80
            || !id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(ConfigError::ValidationError("Invalid environment identifier".into()));
        }
        Ok(())
    }

    pub fn validate_new_id(id: &str) -> Result<(), ConfigError> {
        let bytes = id.as_bytes();
        if bytes.len() != 36 || bytes[14] != b'4' || !matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
            || bytes.iter().enumerate().any(|(index, &byte)| {
                if matches!(index, 8 | 13 | 18 | 23) { byte != b'-' }
                else { !matches!(byte, b'0'..=b'9' | b'a'..=b'f') }
            })
        {
            return Err(ConfigError::ValidationError("Invalid environment UUID".into()));
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
