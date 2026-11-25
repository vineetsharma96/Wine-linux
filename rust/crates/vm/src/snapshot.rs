use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::config::{now_epoch_ms, DiskFormat, VmProfile};
use crate::error::VmError;
use crate::monitor::QmpClient;
use crate::storage::VmStorageManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSnapshotMetadata {
    pub snapshot_id: String,
    pub vm_id: String,
    pub name: String,
    pub description: String,
    pub disk_format: DiskFormat,
    pub mechanism: String,
    pub created_at_epoch_ms: u64,
    pub disk_sha256: Option<String>,
    pub backup_disk_path: Option<PathBuf>,
}

pub struct VmSnapshotManager;

impl VmSnapshotManager {
    pub fn list_snapshots(vm_dir: &Path) -> Vec<VmSnapshotMetadata> {
        let snap_dir = vm_dir.join("snapshots");
        let Ok(entries) = fs::read_dir(&snap_dir) else {
            return Vec::new();
        };
        let mut items = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(raw) = fs::read_to_string(&path) {
                    if let Ok(meta) = serde_json::from_str::<VmSnapshotMetadata>(&raw) {
                        items.push(meta);
                    }
                }
            }
        }
        items.sort_by_key(|s| s.created_at_epoch_ms);
        items
    }

    pub fn create_snapshot(
        vm_dir: &Path,
        profile: &VmProfile,
        snapshot_id: &str,
        name: &str,
        description: &str,
        is_running: bool,
        qemu_img_bin: Option<&Path>,
    ) -> Result<VmSnapshotMetadata, VmError> {
        validate_snapshot_id(snapshot_id)?;
        let primary = profile
            .primary_disk()
            .ok_or_else(|| VmError::Snapshot("VM has no primary disk".to_string()))?;
        let snap_dir = vm_dir.join("snapshots");
        fs::create_dir_all(&snap_dir)?;

        let meta_path = snap_dir.join(format!("{snapshot_id}.json"));
        if meta_path.exists() {
            return Err(VmError::Snapshot(format!(
                "Snapshot '{snapshot_id}' already exists"
            )));
        }

        let (mechanism, backup_disk_path, disk_sha256) = if is_running
            && primary.format == DiskFormat::Qcow2
        {
            QmpClient::human_monitor_command(profile.qmp_port, &format!("savevm {snapshot_id}"))?;
            ("qmp_internal_qcow2".to_string(), None, None)
        } else if primary.format == DiskFormat::Qcow2
            && qemu_img_bin.is_some_and(|p| p.is_file())
        {
            let bin = qemu_img_bin.unwrap();
            let status = std::process::Command::new(bin)
                .args([
                    "snapshot",
                    "-c",
                    snapshot_id,
                    &primary.path.to_string_lossy(),
                ])
                .status()?;
            if !status.success() {
                return Err(VmError::Snapshot(format!(
                    "qemu-img snapshot -c failed with status {status}"
                )));
            }
            ("qemu_img_internal_qcow2".to_string(), None, None)
        } else {
            // Offline atomic copy snapshot with SHA-256 digest (works for both RAW and QCOW2 even without qemu-img)
            if is_running {
                return Err(VmError::Snapshot(
                    "Stop the VM before creating an offline file snapshot on RAW disks".to_string(),
                ));
            }
            let backup_path = snap_dir.join(format!("{snapshot_id}.disk"));
            let part_path = snap_dir.join(format!("{snapshot_id}.disk.part"));
            fs::copy(&primary.path, &part_path)?;
            fs::rename(&part_path, &backup_path)?;
            let hash = VmStorageManager::compute_sha256(&backup_path)?;
            (
                "offline_verified_copy".to_string(),
                Some(backup_path),
                Some(hash),
            )
        };

        let meta = VmSnapshotMetadata {
            snapshot_id: snapshot_id.to_string(),
            vm_id: profile.id.clone(),
            name: if name.trim().is_empty() {
                snapshot_id.to_string()
            } else {
                name.trim().to_string()
            },
            description: description.trim().to_string(),
            disk_format: primary.format,
            mechanism,
            created_at_epoch_ms: now_epoch_ms(),
            disk_sha256,
            backup_disk_path,
        };
        fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
        Ok(meta)
    }

    pub fn restore_snapshot(
        vm_dir: &Path,
        profile: &VmProfile,
        snapshot_id: &str,
        is_running: bool,
        qemu_img_bin: Option<&Path>,
    ) -> Result<(), VmError> {
        validate_snapshot_id(snapshot_id)?;
        let meta_path = vm_dir.join("snapshots").join(format!("{snapshot_id}.json"));
        let raw = fs::read_to_string(&meta_path)
            .map_err(|_| VmError::Snapshot(format!("Snapshot '{snapshot_id}' not found")))?;
        let meta: VmSnapshotMetadata = serde_json::from_str(&raw)?;
        let primary = profile
            .primary_disk()
            .ok_or_else(|| VmError::Snapshot("VM has no primary disk".to_string()))?;

        match meta.mechanism.as_str() {
            "qmp_internal_qcow2" if is_running => {
                QmpClient::human_monitor_command(
                    profile.qmp_port,
                    &format!("loadvm {snapshot_id}"),
                )?;
                Ok(())
            }
            "qemu_img_internal_qcow2" | "qmp_internal_qcow2" => {
                let bin = qemu_img_bin.filter(|p| p.is_file()).ok_or_else(|| {
                    VmError::Unsupported(
                        "Restoring internal QCOW2 snapshot while stopped requires qemu-img"
                            .to_string(),
                    )
                })?;
                let status = std::process::Command::new(bin)
                    .args([
                        "snapshot",
                        "-a",
                        snapshot_id,
                        &primary.path.to_string_lossy(),
                    ])
                    .status()?;
                if !status.success() {
                    return Err(VmError::Snapshot(format!(
                        "qemu-img snapshot -a failed with status {status}"
                    )));
                }
                Ok(())
            }
            _ => {
                if is_running {
                    return Err(VmError::Snapshot(
                        "Stop the VM before restoring an offline disk snapshot".to_string(),
                    ));
                }
                let backup = meta.backup_disk_path.ok_or_else(|| {
                    VmError::Snapshot("Snapshot backup disk path missing".to_string())
                })?;
                if let Some(expected) = meta.disk_sha256 {
                    let actual = VmStorageManager::compute_sha256(&backup)?;
                    if !actual.eq_ignore_ascii_case(&expected) {
                        return Err(VmError::Security(format!(
                            "Snapshot '{snapshot_id}' SHA-256 verification failed"
                        )));
                    }
                }
                let part = PathBuf::from(format!("{}.restore.part", primary.path.display()));
                fs::copy(&backup, &part)?;
                fs::rename(&part, &primary.path)?;
                Ok(())
            }
        }
    }

    pub fn delete_snapshot(
        vm_dir: &Path,
        profile: &VmProfile,
        snapshot_id: &str,
        is_running: bool,
        qemu_img_bin: Option<&Path>,
    ) -> Result<(), VmError> {
        validate_snapshot_id(snapshot_id)?;
        let meta_path = vm_dir.join("snapshots").join(format!("{snapshot_id}.json"));
        let raw = fs::read_to_string(&meta_path)
            .map_err(|_| VmError::Snapshot(format!("Snapshot '{snapshot_id}' not found")))?;
        let meta: VmSnapshotMetadata = serde_json::from_str(&raw)?;
        if let Some(backup) = meta.backup_disk_path {
            let _ = fs::remove_file(backup);
        } else if let Some(primary) = profile.primary_disk() {
            if is_running && meta.mechanism == "qmp_internal_qcow2" {
                let _ = QmpClient::human_monitor_command(
                    profile.qmp_port,
                    &format!("delvm {snapshot_id}"),
                );
            } else if let Some(bin) = qemu_img_bin.filter(|p| p.is_file()) {
                let _ = std::process::Command::new(bin)
                    .args([
                        "snapshot",
                        "-d",
                        snapshot_id,
                        &primary.path.to_string_lossy(),
                    ])
                    .status();
            }
        }
        fs::remove_file(meta_path)?;
        Ok(())
    }
}

fn validate_snapshot_id(id: &str) -> Result<(), VmError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(VmError::Snapshot(format!(
            "Invalid snapshot id '{id}'; use alphanumeric, dash, or underscore"
        )));
    }
    Ok(())
}
