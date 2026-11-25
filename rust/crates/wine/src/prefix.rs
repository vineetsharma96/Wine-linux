use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use tar::{Archive, Builder};

use androidpc_graphics_backend::GraphicsBackendKind;

use crate::config::WineArch;
use crate::error::WineError;
use crate::manager::WINE_PINNED_VERSION;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinePrefixMetadata {
    pub id: String,
    pub display_name: String,
    pub environment_id: String,
    pub wine_version: String,
    pub windows_version: String,
    pub arch: WineArch,
    /// Guest path inside the Linux rootfs (e.g. `/home/user/.androidpc-prefixes/<id>`).
    pub prefix_path: String,
    pub env_vars: BTreeMap<String, String>,
    pub dll_overrides: BTreeMap<String, String>,
    pub graphics_backend: GraphicsBackendKind,
    pub dxvk_enabled: bool,
    pub vkd3d_enabled: bool,
    pub installed_apps: Vec<String>,
    pub dependencies: Vec<String>,
    pub created_at_epoch_ms: u64,
    pub last_used_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinePrefixBackupManifest {
    pub backup_id: String,
    pub prefix_id: String,
    pub prefix_name: String,
    pub environment_id: String,
    pub archive_filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub created_at_epoch_ms: u64,
    pub metadata: WinePrefixMetadata,
}

/// Manages isolated Wine prefixes per environment without allowing applications
/// to accidentally share or corrupt unrelated prefix directories.
pub struct WinePrefixManager {
    registry_dir: PathBuf,
    backups_dir: PathBuf,
}

impl WinePrefixManager {
    pub fn new(base_data_dir: &Path) -> Self {
        Self {
            registry_dir: base_data_dir.join("wine_prefixes"),
            backups_dir: base_data_dir.join("wine_prefix_backups"),
        }
    }

    fn now_epoch_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn validate_prefix_id(id: &str) -> Result<(), WineError> {
        let valid = !id.is_empty()
            && id.len() <= 64
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'));
        if !valid {
            return Err(WineError::InvalidConfig(format!(
                "Invalid prefix ID '{id}': must be 1..=64 ASCII alphanumeric, '-' or '_' characters"
            )));
        }
        Ok(())
    }

    fn metadata_file(&self, id: &str) -> PathBuf {
        self.registry_dir.join(format!("{id}.json"))
    }

    pub fn guest_prefix_host_dir(rootfs_dir: &Path, guest_prefix_path: &str) -> Result<PathBuf, WineError> {
        let rel = guest_prefix_path.trim().trim_start_matches('/');
        if rel.is_empty() {
            return Err(WineError::InvalidConfig("Empty Wine prefix path".into()));
        }
        Ok(androidpc_security::resolve_safe_path(rootfs_dir, Path::new(rel))?)
    }

    pub fn create_prefix(
        &self,
        id: &str,
        display_name: &str,
        environment_id: &str,
        rootfs_dir: Option<&Path>,
        arch: WineArch,
        windows_version: &str,
        graphics_backend: GraphicsBackendKind,
    ) -> Result<WinePrefixMetadata, WineError> {
        Self::validate_prefix_id(id)?;
        fs::create_dir_all(&self.registry_dir)?;
        let meta_path = self.metadata_file(id);
        if meta_path.exists() {
            return Err(WineError::PrefixAlreadyExists(id.to_string()));
        }

        let guest_path = format!("/home/user/.androidpc-prefixes/{id}");
        if let Some(rootfs) = rootfs_dir {
            let host_dir = Self::guest_prefix_host_dir(rootfs, &guest_path)?;
            fs::create_dir_all(host_dir.join("drive_c/windows/system32"))?;
            fs::create_dir_all(host_dir.join("drive_c/Program Files"))?;
            fs::create_dir_all(host_dir.join("drive_c/users/user/Desktop"))?;
        }

        let now = Self::now_epoch_ms();
        let meta = WinePrefixMetadata {
            id: id.to_string(),
            display_name: if display_name.trim().is_empty() {
                id.to_string()
            } else {
                display_name.trim().to_string()
            },
            environment_id: environment_id.to_string(),
            wine_version: WINE_PINNED_VERSION.to_string(),
            windows_version: if windows_version.trim().is_empty() {
                "win10".to_string()
            } else {
                windows_version.trim().to_string()
            },
            arch,
            prefix_path: guest_path,
            env_vars: BTreeMap::new(),
            dll_overrides: BTreeMap::from([
                ("mscoree".to_string(), "".to_string()),
                ("mshtml".to_string(), "".to_string()),
            ]),
            graphics_backend,
            dxvk_enabled: false,
            vkd3d_enabled: false,
            installed_apps: Vec::new(),
            dependencies: Vec::new(),
            created_at_epoch_ms: now,
            last_used_at_epoch_ms: now,
        };

        fs::write(&meta_path, serde_json::to_vec_pretty(&meta)?)?;
        Ok(meta)
    }

    pub fn list_prefixes(&self) -> Vec<WinePrefixMetadata> {
        let Ok(entries) = fs::read_dir(&self.registry_dir) else {
            return Vec::new();
        };
        let mut list = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(raw) = fs::read_to_string(&path) {
                    if let Ok(meta) = serde_json::from_str::<WinePrefixMetadata>(&raw) {
                        list.push(meta);
                    }
                }
            }
        }
        list.sort_by(|a, b| a.created_at_epoch_ms.cmp(&b.created_at_epoch_ms));
        list
    }

    pub fn get_prefix(&self, id: &str) -> Result<WinePrefixMetadata, WineError> {
        Self::validate_prefix_id(id)?;
        let path = self.metadata_file(id);
        if !path.is_file() {
            return Err(WineError::PrefixNotFound(id.to_string()));
        }
        let raw = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    pub fn save_prefix(&self, meta: &WinePrefixMetadata) -> Result<(), WineError> {
        Self::validate_prefix_id(&meta.id)?;
        fs::create_dir_all(&self.registry_dir)?;
        fs::write(self.metadata_file(&meta.id), serde_json::to_vec_pretty(meta)?)?;
        Ok(())
    }

    pub fn touch_last_used(&self, id: &str) -> Result<WinePrefixMetadata, WineError> {
        let mut meta = self.get_prefix(id)?;
        meta.last_used_at_epoch_ms = Self::now_epoch_ms();
        self.save_prefix(&meta)?;
        Ok(meta)
    }

    pub fn delete_prefix(&self, id: &str, rootfs_dir: Option<&Path>) -> Result<(), WineError> {
        let meta = self.get_prefix(id)?;
        if let Some(rootfs) = rootfs_dir {
            let host_dir = Self::guest_prefix_host_dir(rootfs, &meta.prefix_path)?;
            if host_dir.exists() {
                fs::remove_dir_all(&host_dir)?;
            }
        }
        let meta_path = self.metadata_file(id);
        if meta_path.exists() {
            fs::remove_file(meta_path)?;
        }
        Ok(())
    }

    pub fn clone_prefix(
        &self,
        source_id: &str,
        new_id: &str,
        new_display_name: &str,
        rootfs_dir: Option<&Path>,
    ) -> Result<WinePrefixMetadata, WineError> {
        Self::validate_prefix_id(new_id)?;
        let source = self.get_prefix(source_id)?;
        if self.metadata_file(new_id).exists() {
            return Err(WineError::PrefixAlreadyExists(new_id.to_string()));
        }

        let new_guest_path = format!("/home/user/.androidpc-prefixes/{new_id}");
        if let Some(rootfs) = rootfs_dir {
            let src_dir = Self::guest_prefix_host_dir(rootfs, &source.prefix_path)?;
            let dst_dir = Self::guest_prefix_host_dir(rootfs, &new_guest_path)?;
            if src_dir.is_dir() {
                copy_dir_recursive(&src_dir, &dst_dir)?;
            } else {
                fs::create_dir_all(dst_dir.join("drive_c/windows/system32"))?;
            }
        }

        let now = Self::now_epoch_ms();
        let mut cloned = source;
        cloned.id = new_id.to_string();
        cloned.display_name = if new_display_name.trim().is_empty() {
            format!("{} (Clone)", cloned.display_name)
        } else {
            new_display_name.trim().to_string()
        };
        cloned.prefix_path = new_guest_path;
        cloned.created_at_epoch_ms = now;
        cloned.last_used_at_epoch_ms = now;

        self.save_prefix(&cloned)?;
        Ok(cloned)
    }

    /// Creates a gzipped tar backup of an isolated Wine prefix with a SHA-256 manifest.
    pub fn backup_prefix(
        &self,
        id: &str,
        rootfs_dir: &Path,
    ) -> Result<WinePrefixBackupManifest, WineError> {
        let meta = self.get_prefix(id)?;
        let src_dir = Self::guest_prefix_host_dir(rootfs_dir, &meta.prefix_path)?;
        if !src_dir.is_dir() {
            return Err(WineError::PrefixNotFound(format!(
                "Prefix directory missing on disk: {}",
                src_dir.display()
            )));
        }

        fs::create_dir_all(&self.backups_dir)?;
        let now = Self::now_epoch_ms();
        let backup_id = format!("wpbk-{id}-{now}");
        let archive_filename = format!("{backup_id}.tar.gz");
        let archive_path = self.backups_dir.join(&archive_filename);

        {
            let tar_gz = File::create(&archive_path)?;
            let enc = GzEncoder::new(tar_gz, Compression::default());
            let mut tar = Builder::new(enc);
            tar.append_dir_all(".", &src_dir)?;
            let enc = tar.into_inner()?;
            enc.finish()?;
        }

        let sha256 = androidpc_security::compute_file_sha256(&archive_path)?;
        let size_bytes = fs::metadata(&archive_path)?.len();
        let manifest = WinePrefixBackupManifest {
            backup_id: backup_id.clone(),
            prefix_id: meta.id.clone(),
            prefix_name: meta.display_name.clone(),
            environment_id: meta.environment_id.clone(),
            archive_filename,
            sha256,
            size_bytes,
            created_at_epoch_ms: now,
            metadata: meta,
        };
        fs::write(
            self.backups_dir.join(format!("{backup_id}.manifest.json")),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        Ok(manifest)
    }

    pub fn list_backups(&self) -> Vec<WinePrefixBackupManifest> {
        let Ok(entries) = fs::read_dir(&self.backups_dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".manifest.json"))
            {
                if let Ok(raw) = fs::read_to_string(&p) {
                    if let Ok(manifest) = serde_json::from_str::<WinePrefixBackupManifest>(&raw) {
                        out.push(manifest);
                    }
                }
            }
        }
        out.sort_by(|a, b| b.created_at_epoch_ms.cmp(&a.created_at_epoch_ms));
        out
    }

    /// Restores a Wine prefix backup into a new isolated prefix ID after verifying SHA-256
    /// integrity and defending every archive entry against Zip-Slip / path traversal.
    pub fn restore_prefix(
        &self,
        backup_id: &str,
        target_prefix_id: &str,
        rootfs_dir: &Path,
    ) -> Result<WinePrefixMetadata, WineError> {
        Self::validate_prefix_id(target_prefix_id)?;
        if self.metadata_file(target_prefix_id).exists() {
            return Err(WineError::PrefixAlreadyExists(target_prefix_id.to_string()));
        }

        let manifest_path = self.backups_dir.join(format!("{backup_id}.manifest.json"));
        if !manifest_path.is_file() {
            return Err(WineError::PrefixNotFound(format!(
                "Backup manifest not found: {backup_id}"
            )));
        }
        let manifest: WinePrefixBackupManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
        let archive_path = self.backups_dir.join(&manifest.archive_filename);
        androidpc_security::verify_file_sha256(&archive_path, &manifest.sha256)?;

        let new_guest_path = format!("/home/user/.androidpc-prefixes/{target_prefix_id}");
        let dst_dir = Self::guest_prefix_host_dir(rootfs_dir, &new_guest_path)?;
        fs::create_dir_all(&dst_dir)?;

        let file = File::open(&archive_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        for entry_res in archive.entries()? {
            let mut entry = entry_res?;
            let entry_path = entry.path()?.into_owned();
            let safe_target = androidpc_security::resolve_safe_path(&dst_dir, &entry_path)?;
            if entry.header().entry_type().is_dir() {
                fs::create_dir_all(&safe_target)?;
            } else if entry.header().entry_type().is_file() {
                if let Some(parent) = safe_target.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out = File::create(&safe_target)?;
                std::io::copy(&mut entry, &mut out)?;
            }
        }

        let now = Self::now_epoch_ms();
        let mut restored = manifest.metadata;
        restored.id = target_prefix_id.to_string();
        restored.display_name = format!("{} (Restored)", restored.display_name);
        restored.prefix_path = new_guest_path;
        restored.created_at_epoch_ms = now;
        restored.last_used_at_epoch_ms = now;
        self.save_prefix(&restored)?;
        Ok(restored)
    }

    /// Verifies and repairs required directory skeleton inside an isolated prefix.
    pub fn repair_prefix_structure(
        &self,
        id: &str,
        rootfs_dir: &Path,
    ) -> Result<Vec<String>, WineError> {
        let meta = self.get_prefix(id)?;
        let host_dir = Self::guest_prefix_host_dir(rootfs_dir, &meta.prefix_path)?;
        let required = [
            "drive_c",
            "drive_c/windows",
            "drive_c/windows/system32",
            "drive_c/windows/syswow64",
            "drive_c/Program Files",
            "drive_c/users/user/Desktop",
            "dosdevices",
        ];
        let mut repaired = Vec::new();
        for rel in required {
            let dir = host_dir.join(rel);
            if !dir.is_dir() {
                fs::create_dir_all(&dir)?;
                repaired.push(format!("Created missing directory: {rel}"));
            }
        }
        Ok(repaired)
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
