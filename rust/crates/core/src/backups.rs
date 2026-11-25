use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use androidpc_configuration::EnvironmentConfig;
use androidpc_downloader::verifier::Sha256Verifier;
use androidpc_linux_runtime::ProotEnvironmentManager;
use androidpc_runtime_api::{Architecture, EnvironmentManager, EnvironmentState};
use serde::{Deserialize, Serialize};

use crate::{packages::PackageJob, CoreError, EngineCore};

#[derive(Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub created_at_epoch_ms: u64,
    pub size_bytes: u64,
    pub source_name: String,
    pub distribution: String,
}

#[derive(Serialize, Deserialize)]
struct BackupRecord {
    schema_version: u32,
    info: BackupInfo,
    config: EnvironmentConfig,
    sha256: String,
}

fn timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|value| value.as_millis() as u64).unwrap_or(0)
}

fn failure(message: &str) -> CoreError {
    CoreError::SubsystemFailure(message.to_owned())
}

fn check_cancelled(job: &PackageJob) -> Result<(), CoreError> {
    if job.cancelled.load(Ordering::Acquire) {
        return Err(failure("Operation cancelled"));
    }
    Ok(())
}

struct CheckedWriter<'a> {
    file: &'a mut File,
    job: &'a PackageJob,
}

impl Write for CheckedWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.job.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::other("Backup cancelled"));
        }
        self.file.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

fn load_record(directory: &Path, id: &str) -> Result<BackupRecord, CoreError> {
    if !directory.symlink_metadata()?.is_dir() {
        return Err(failure("Invalid backup directory"));
    }
    let meta = directory.join("manifest.json");
    let info = meta.symlink_metadata()?;
    if !info.is_file() || info.len() > 64 * 1024 {
        return Err(failure("Invalid backup metadata"));
    }
    let record: BackupRecord = serde_json::from_slice(&fs::read(meta)?)?;
    record.config.validate()?;
    if record.schema_version != 1 || record.info.id != id || record.info.size_bytes == 0
        || record.sha256.len() != 64 || !record.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(failure("Invalid backup manifest"));
    }
    Ok(record)
}

impl EngineCore {
    pub fn list_backups(&self, source_id: &str) -> Result<Vec<BackupInfo>, CoreError> {
        let mut items = Vec::new();
        for entry in fs::read_dir(self.storage.backup_root()?)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(id) = name.to_str() else { continue };
            if EnvironmentConfig::validate_new_id(id).is_err() { continue; }
            if let Ok(record) = load_record(&entry.path(), id) {
                if source_id.is_empty() || record.config.id == source_id {
                    items.push(record.info);
                }
            }
        }
        items.sort_by_key(|item| std::cmp::Reverse(item.created_at_epoch_ms));
        Ok(items)
    }

    pub fn create_backup(&self, source_id: &str, backup_id: &str) -> Result<BackupInfo, CoreError> {
        EnvironmentConfig::validate_new_id(backup_id)?;
        let job = Arc::new(PackageJob::new(EnvironmentState::BackingUp));
        let config = {
            let environments = self.environments.write().map_err(|_| failure("Environment lock unavailable"))?;
            let manager = environments.get(source_id).ok_or_else(|| failure("Environment not installed"))?;
            if manager.is_running() || self.has_live_terminal(source_id)? || self.has_live_desktop(source_id)? {
                return Err(failure("Stop the environment before making a backup"));
            }
            self.ensure_no_package_operation(source_id)?;
            self.package_jobs.lock().map_err(|_| failure("Operation lock unavailable"))?
                .insert(source_id.to_owned(), job.clone());
            manager.config().clone()
        };
        let result = self.write_backup(backup_id, config, &job);
        if let Ok(mut jobs) = self.package_jobs.lock() { jobs.remove(source_id); }
        result
    }

    fn write_backup(&self, backup_id: &str, config: EnvironmentConfig, job: &PackageJob) -> Result<BackupInfo, CoreError> {
        let parent = self.storage.backup_root()?;
        #[cfg(unix)]
        {
            let root = config.rootfs_directory.parent().ok_or_else(|| failure("Missing guest directory"))?;
            let estimate = self.storage.environment_storage_bytes(root)?;
            if crate::diagnostics::query_available_storage_bytes(&parent) < estimate.saturating_add(estimate / 5) {
                return Err(failure("Free up storage before making this backup"));
            }
        }
        let final_dir = parent.join(backup_id);
        if final_dir.symlink_metadata().is_ok() { return Err(failure("Backup already exists")); }
        let staging = parent.join(format!(".backup-{backup_id}"));
        fs::create_dir(&staging)?;
        let result = (|| {
            let path = staging.join("rootfs.tar");
            let mut file = OpenOptions::new().write(true).create_new(true).open(&path)?;
            self.storage.write_rootfs_archive(&config.rootfs_directory, CheckedWriter { file: &mut file, job })?;
            file.sync_all()?;
            let size_bytes = file.metadata()?.len();
            drop(file);
            check_cancelled(job)?;
            let digest = Sha256Verifier::hash_file(&path)?;
            let info = BackupInfo {
                id: backup_id.to_owned(), created_at_epoch_ms: timestamp(), size_bytes,
                source_name: config.display_name.clone().unwrap_or_else(|| config.distribution.clone()),
                distribution: config.distribution.clone(),
            };
            let record = BackupRecord { schema_version: 1, info: info.clone(), config, sha256: digest };
            let mut manifest = File::create(staging.join("manifest.json"))?;
            manifest.write_all(&serde_json::to_vec_pretty(&record)?)?;
            manifest.sync_all()?;
            drop(manifest);
            check_cancelled(job)?;
            fs::rename(&staging, final_dir)?;
            Ok(info)
        })();
        if result.is_err() { let _ = fs::remove_dir_all(&staging); }
        result
    }

    pub fn restore_backup(&self, backup_id: &str, id: &str, name: &str) -> Result<EnvironmentConfig, CoreError> {
        EnvironmentConfig::validate_new_id(backup_id)?;
        EnvironmentConfig::validate_new_id(id)?;
        let _install = self.environment_install.try_lock().map_err(|_| failure("Another install or restore is active"))?;
        let directory = self.storage.backup_root()?.join(backup_id);
        let record = load_record(&directory, backup_id)?;
        let job = Arc::new(PackageJob::new(EnvironmentState::Restoring));
        {
            let environments = self.environments.write().map_err(|_| failure("Environment lock unavailable"))?;
            if environments.contains_key(id) { return Err(failure("Environment already exists")); }
            self.ensure_no_package_operation(id)?;
            self.package_jobs.lock().map_err(|_| failure("Operation lock unavailable"))?
                .insert(id.to_owned(), job.clone());
        }
        let result = self.restore_verified(directory.join("rootfs.tar").as_path(), record, id, name, &job);
        if let Ok(mut jobs) = self.package_jobs.lock() { jobs.remove(id); }
        result
    }

    fn restore_verified(&self, archive: &Path, record: BackupRecord, id: &str, name: &str, job: &PackageJob) -> Result<EnvironmentConfig, CoreError> {
        let meta = archive.symlink_metadata()?;
        if !meta.is_file() || meta.len() != record.info.size_bytes || !self.verify_archive(archive, &record.sha256)? {
            return Err(failure("Backup size or SHA-256 is incorrect"));
        }
        let mut config = record.config;
        if config.architecture != Architecture::host() || !matches!(config.distribution.as_str(), "ubuntu" | "debian") {
            return Err(failure("Backup is incompatible with this device"));
        }
        let final_dir = self.storage.distribution_root(&config.distribution)?.join(id);
        if ["ubuntu", "debian"].into_iter().filter_map(|distro| self.storage.distribution_root(distro).ok())
            .any(|directory| directory.join(id).symlink_metadata().is_ok())
            || self.storage.environment_root(id).symlink_metadata().is_ok()
        {
            return Err(failure("Environment already exists"));
        }
        #[cfg(unix)]
        if crate::diagnostics::query_available_storage_bytes(&self.config.base_data_dir)
            < meta.len().saturating_add(meta.len() / 5)
        {
            return Err(failure("Free up storage before restoring this backup"));
        }
        config.id = id.to_owned();
        config.display_name = Some(name.trim().to_owned());
        config.rootfs_directory = final_dir.join("rootfs");
        config.created_at_epoch_ms = Some(timestamp());
        config.last_started_at_epoch_ms = None;
        config.custom_mounts.clear();
        config.validate()?;
        check_cancelled(job)?;
        let staging = final_dir.with_file_name(format!(".install-{id}"));
        if let Some(parent) = staging.parent() { fs::create_dir_all(parent)?; }
        fs::create_dir(&staging)?;
        let result = (|| {
            let mut staged = config.clone();
            staged.rootfs_directory = staging.join("rootfs");
            self.extract_rootfs_as(archive, &staged.rootfs_directory, "rootfs.tar")?;
            check_cancelled(job)?;
            let provider = self.resolve_proot_provider();
            let mut manager = ProotEnvironmentManager::new(staged, provider.clone())
                .with_artifacts_dir(&self.config.runtime_artifacts_dir);
            manager.initialize()?;
            if manager.verify_guest(&config.distribution)? != config.version {
                return Err(failure("Restored guest release does not match the backup"));
            }
            config.verified_at_epoch_ms = Some(timestamp());
            config.runtime_version = crate::diagnostics::probe_proot_version(&self.config);
            self.storage.save_config_at(&staging, &serde_json::to_vec_pretty(&config)?)?;
            check_cancelled(job)?;
            let mut environments = self.environments.write().map_err(|_| failure("Environment lock unavailable"))?;
            if environments.contains_key(id) || final_dir.symlink_metadata().is_ok() {
                return Err(failure("Environment already exists"));
            }
            fs::rename(&staging, &final_dir)?;
            environments.insert(id.to_owned(), ProotEnvironmentManager::from_existing(config.clone(), provider)
                .with_artifacts_dir(&self.config.runtime_artifacts_dir));
            Ok(config)
        })();
        if result.is_err() { let _ = fs::remove_dir_all(&staging); }
        result
    }

    pub fn clone_environment(&self, source: &str, id: &str, name: &str, temporary_backup: &str) -> Result<EnvironmentConfig, CoreError> {
        self.create_backup(source, temporary_backup)?;
        let result = self.restore_backup(temporary_backup, id, name);
        if let Ok(dir) = self.storage.backup_root() {
            if let Err(error) = fs::remove_dir_all(dir.join(temporary_backup)) {
                tracing::warn!("Could not clean temporary clone backup: {error}");
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use androidpc_configuration::EngineConfig;

    #[test]
    fn backup_round_trip_is_independent_and_corruption_blocks_restore() {
        let base = std::env::temp_dir().join(format!("androidpc-backup-{}", std::process::id()));
        let cfg = EngineConfig { base_data_dir: base.clone(), ..EngineConfig::default() };
        let engine = EngineCore::new(cfg).unwrap();
        let config = engine.provision_environment("source", "debian").unwrap();
        let marker = config.rootfs_directory.join("home/user/marker");
        fs::write(&marker, b"source data").unwrap();
        engine.package_jobs.lock().unwrap().insert("source".into(), Arc::new(PackageJob::new(EnvironmentState::BackingUp)));
        assert!(engine.start_environment("source").is_err());
        assert!(engine.stop_environment("source").is_err());
        assert!(engine.delete_environment("source").is_err());
        engine.package_jobs.lock().unwrap().remove("source");
        let id = "02bb18dd-2a34-40a6-963b-3712b0deaa84";
        let target = "02bb18dd-2a34-40a6-963b-3712b0deaa85";
        engine.create_backup("source", id).unwrap();
        assert_eq!(engine.list_backups("").unwrap().len(), 1);
        let archive = base.join("backups").join(id).join("rootfs.tar");
        let copy = base.join("copy");
        engine.extract_rootfs(&archive, &copy).unwrap();
        let decoded = copy.join("home/user/marker");
        assert_eq!(fs::read(&decoded).unwrap(), b"source data");
        fs::write(decoded, b"different").unwrap();
        assert_eq!(fs::read(&marker).unwrap(), b"source data");

        OpenOptions::new().write(true).open(&archive).unwrap().write_all(b"!").unwrap();
        assert!(engine.restore_backup(id, target, "Test restore").unwrap_err().to_string().contains("SHA-256"));
        assert!(!engine.storage.distribution_root("debian").unwrap().join(target).exists());
        let abandoned = base.join("backups").join(format!(".backup-{target}"));
        fs::create_dir(&abandoned).unwrap();
        engine.storage.clean_staging_on_startup();
        assert!(!abandoned.exists());
        assert!(marker.is_file());
        fs::remove_dir_all(&base).unwrap();
    }
}
