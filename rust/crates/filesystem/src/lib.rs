pub mod extractor;
pub mod security;

use std::io::Write;
use std::path::{Path, PathBuf};
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

    pub fn backup_root(&self) -> Result<PathBuf, FilesystemError> {
        let root = self.base_dir.join("backups");
        match root.symlink_metadata() {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(FilesystemError::Aborted("Invalid backup directory".into())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir_all(&root)?,
            Err(error) => return Err(error.into()),
        }
        Ok(root)
    }

    pub fn write_rootfs_archive(
        &self,
        rootfs: &Path,
        writer: impl Write,
    ) -> Result<(), FilesystemError> {
        if !rootfs.symlink_metadata()?.is_dir() {
            return Err(FilesystemError::Aborted("Invalid root filesystem".into()));
        }
        let mut archive = tar::Builder::new(writer);
        archive.follow_symlinks(false);
        archive.append_dir_all(".", rootfs)?;
        archive.finish()?;
        Ok(())
    }

    pub fn distribution_root(&self, distribution: &str) -> Result<PathBuf, FilesystemError> {
        if !matches!(distribution, "ubuntu" | "debian") {
            return Err(FilesystemError::Aborted("Unsupported distribution directory".into()));
        }
        let root = self.base_dir.join("environments");
        let distribution_root = root.join(distribution);
        for path in [&root, &distribution_root] {
            match path.symlink_metadata() {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => return Err(FilesystemError::Aborted("Invalid environment directory".into())),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(distribution_root)
    }

    pub fn environment_storage_bytes(&self, root: &Path) -> Result<u64, FilesystemError> {
        if !root.symlink_metadata()?.is_dir() {
            return Err(FilesystemError::Aborted("Invalid environment directory".into()));
        }
        let mut directories = vec![root.to_owned()];
        let mut bytes = 0u64;
        #[cfg(unix)]
        let mut seen = std::collections::HashSet::new();
        while let Some(directory) = directories.pop() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                bytes = bytes.saturating_add(directory.metadata()?.blocks().saturating_mul(512));
            }
            for entry in std::fs::read_dir(directory)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                if kind.is_dir() {
                    directories.push(entry.path());
                } else {
                    let metadata = std::fs::symlink_metadata(entry.path())?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if metadata.nlink() <= 1 || seen.insert((metadata.dev(), metadata.ino())) {
                            bytes = bytes.saturating_add(metadata.blocks().saturating_mul(512));
                        }
                    }
                    #[cfg(not(unix))]
                    if kind.is_file() {
                        bytes = bytes.saturating_add(metadata.len());
                    }
                }
            }
        }
        Ok(bytes)
    }

    pub fn clean_staging_on_startup(&self) {
        for directory in self.registry_directories() {
            self.clean_staging_in(&directory);
        }
        let backup_root = self.base_dir.join("backups");
        if backup_root.symlink_metadata().is_ok_and(|meta| meta.is_dir()) {
            self.clean_staging_in(&backup_root);
        }
    }

    fn clean_staging_in(&self, directory: &Path) {
        let Ok(entries) = std::fs::read_dir(directory) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let id = name.strip_prefix(".install-").or_else(|| name.strip_prefix(".deleted-"))
                .or_else(|| name.strip_prefix(".backup-"));
            if id.is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')))
                && entry.file_type().is_ok_and(|kind| kind.is_dir())
            {
                if let Err(error) = std::fs::remove_dir_all(entry.path()) {
                    tracing::warn!("Could not remove interrupted environment staging: {error}");
                }
            }
        }
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
        self.save_config_at(&env_root, json.as_bytes())
    }

    pub fn save_config_at(&self, directory: &Path, json: &[u8]) -> Result<(), FilesystemError> {
        if !directory.symlink_metadata()?.is_dir() {
            return Err(FilesystemError::Aborted("Invalid config directory".into()));
        }
        let temp = directory.join(".config.json.tmp");
        // The registry lock serializes configuration writes.
        match std::fs::remove_file(&temp) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let result = (|| {
            let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(&temp)?;
            file.write_all(json)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp, directory.join("config.json"))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temp);
        }
        result
    }

    fn registry_directories(&self) -> Vec<PathBuf> {
        let legacy = self.base_dir.join("environments");
        if !legacy.symlink_metadata().is_ok_and(|meta| meta.is_dir()) {
            return Vec::new();
        }
        let mut roots = vec![legacy];
        roots.extend(["ubuntu", "debian"].into_iter().filter_map(|distro| self.distribution_root(distro).ok()));
        roots
    }

    pub fn list_saved_config_jsons(&self) -> Vec<(PathBuf, String)> {
        let mut results = Vec::new();
        for directory in self.registry_directories() {
            let Ok(entries) = std::fs::read_dir(directory) else { continue };
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with('.')
                    || !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    continue;
                }
                let cfg_path = entry.path().join("config.json");
                if cfg_path.symlink_metadata().is_ok_and(|meta| meta.is_file() && meta.len() <= 64 * 1024) {
                    if let Ok(contents) = std::fs::read_to_string(cfg_path) {
                        results.push((entry.path(), contents));
                    }
                }
            }
        }
        results
    }
}
