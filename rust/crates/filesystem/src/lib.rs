pub mod extractor;
pub mod security;

use std::path::PathBuf;
use thiserror::Error;

pub use extractor::ArchiveExtractor;
pub use security::PathSecurity;

#[derive(Debug, Error)]
pub enum FilesystemError {
    #[error("IO failure: {0}")]
    Io(#[from] std::io::Error),

    #[error("Security violation: path traversal attempt detected: target '{target}' escapes root '{base}'")]
    PathTraversalAttempt {
        base: PathBuf,
        target: PathBuf,
    },

    #[error("Invalid archive format: {0}")]
    InvalidArchive(String),

    #[error("Target directory does not exist: {0}")]
    DirectoryNotFound(PathBuf),

    #[error("Operation aborted: {0}")]
    Aborted(String),
}

pub struct StorageManager {
    base_dir: PathBuf,
}

impl StorageManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    pub fn environment_root(&self, env_id: &str) -> PathBuf {
        self.base_dir.join("environments").join(env_id)
    }

    pub fn environment_rootfs(&self, env_id: &str) -> PathBuf {
        self.environment_root(env_id).join("rootfs")
    }

    pub fn ensure_environment_directories(&self, env_id: &str) -> Result<PathBuf, FilesystemError> {
        let rootfs = self.environment_rootfs(env_id);
        std::fs::create_dir_all(&rootfs)?;
        Ok(rootfs)
    }

    pub fn remove_environment(&self, env_id: &str) -> Result<(), FilesystemError> {
        let env_root = self.environment_root(env_id);
        if env_root.exists() {
            std::fs::remove_dir_all(&env_root)?;
        }
        Ok(())
    }

    pub fn environment_config_path(&self, env_id: &str) -> PathBuf {
        self.environment_root(env_id).join("config.json")
    }

    pub fn save_environment_config(&self, env_id: &str, json: &str) -> Result<(), FilesystemError> {
        let env_root = self.environment_root(env_id);
        std::fs::create_dir_all(&env_root)?;
        std::fs::write(self.environment_config_path(env_id), json)?;
        Ok(())
    }

    pub fn list_saved_config_jsons(&self) -> Vec<String> {
        let envs_dir = self.base_dir.join("environments");
        let mut results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(envs_dir) {
            for entry in entries.flatten() {
                let cfg_path = entry.path().join("config.json");
                if cfg_path.is_file() {
                    if let Ok(contents) = std::fs::read_to_string(cfg_path) {
                        results.push(contents);
                    }
                }
            }
        }
        results
    }
}
