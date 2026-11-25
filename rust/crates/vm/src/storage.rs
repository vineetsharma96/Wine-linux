use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{DiskBus, DiskFormat, StorageDeviceConfig};
use crate::error::VmError;

/// QCOW2 v3 magic bytes: "QFI\xfb"
pub const QCOW2_MAGIC: [u8; 4] = [0x51, 0x46, 0x49, 0xFB];
const QCOW2_CLUSTER_BITS: u32 = 16; // 64 KiB clusters
const QCOW2_CLUSTER_SIZE: u64 = 1u64 << QCOW2_CLUSTER_BITS;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInspectionReport {
    pub path: String,
    pub exists: bool,
    pub detected_format: String,
    pub virtual_size_bytes: u64,
    pub actual_disk_bytes: u64,
    pub qcow2_version: Option<u32>,
    pub healthy: bool,
    pub diagnostics: Vec<String>,
}

pub struct VmStorageManager;

impl VmStorageManager {
    /// Creates a new virtual disk image (QCOW2 v3 or sparse RAW) atomically via a `.part` staging file.
    pub fn create_disk(
        path: &Path,
        format: DiskFormat,
        size_mb: u64,
        qemu_img_bin: Option<&Path>,
    ) -> Result<DiskInspectionReport, VmError> {
        if size_mb < 64 {
            return Err(VmError::Storage(format!(
                "Requested disk size {size_mb} MB is below minimum 64 MB"
            )));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let virtual_bytes = size_mb
            .checked_mul(1024 * 1024)
            .ok_or_else(|| VmError::Storage("Disk size overflow".to_string()))?;

        let part_path = PathBuf::from(format!("{}.part", path.display()));
        let _ = fs::remove_file(&part_path);

        match format {
            DiskFormat::Raw => {
                let file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&part_path)?;
                file.set_len(virtual_bytes)?;
                file.sync_all()?;
            }
            DiskFormat::Qcow2 => {
                let used_qemu_img = if let Some(bin) = qemu_img_bin.filter(|p| p.is_file()) {
                    std::process::Command::new(bin)
                        .args([
                            "create",
                            "-f",
                            "qcow2",
                            &part_path.to_string_lossy(),
                            &format!("{size_mb}M"),
                        ])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                } else {
                    false
                };

                if !used_qemu_img {
                    Self::write_minimal_qcow2_v3(&part_path, virtual_bytes)?;
                }
            }
        }

        fs::rename(&part_path, path)?;
        Self::inspect_disk(path)
    }

    /// Writes a specification-compliant empty QCOW2 v3 image (64 KiB clusters, valid refcount table + L1 table)
    /// directly in pure Rust so QCOW2 creation works even before `qemu-img` is staged.
    pub fn write_minimal_qcow2_v3(path: &Path, virtual_bytes: u64) -> Result<(), VmError> {
        // Cluster 0: Header (104 bytes padded to 64 KiB)
        // Cluster 1: Refcount Table (first entry points to Cluster 2)
        // Cluster 2: Refcount Block (marks clusters 0, 1, 2, 3 as refcount=1)
        // Cluster 3: L1 Table (all zero -> sparse unallocated clusters)
        let l2_entries = QCOW2_CLUSTER_SIZE / 8; // 8192 entries per L2 table
        let bytes_per_l1_entry = l2_entries * QCOW2_CLUSTER_SIZE; // 512 MiB per L1 entry
        let l1_size = ((virtual_bytes.saturating_add(bytes_per_l1_entry - 1)) / bytes_per_l1_entry)
            .max(1) as u32;
        let l1_clusters = ((l1_size as u64 * 8) + QCOW2_CLUSTER_SIZE - 1) / QCOW2_CLUSTER_SIZE;
        let total_meta_clusters = 3 + l1_clusters;

        let mut header = vec![0u8; QCOW2_CLUSTER_SIZE as usize];
        header[0..4].copy_from_slice(&QCOW2_MAGIC);
        header[4..8].copy_from_slice(&3u32.to_be_bytes()); // version 3
        header[8..16].copy_from_slice(&0u64.to_be_bytes()); // backing_file_offset
        header[16..20].copy_from_slice(&0u32.to_be_bytes()); // backing_file_size
        header[20..24].copy_from_slice(&QCOW2_CLUSTER_BITS.to_be_bytes());
        header[24..32].copy_from_slice(&virtual_bytes.to_be_bytes()); // size
        header[32..36].copy_from_slice(&0u32.to_be_bytes()); // crypt_method
        header[36..40].copy_from_slice(&l1_size.to_be_bytes()); // l1_size
        header[40..48].copy_from_slice(&(3 * QCOW2_CLUSTER_SIZE).to_be_bytes()); // l1_table_offset
        header[48..56].copy_from_slice(&QCOW2_CLUSTER_SIZE.to_be_bytes()); // refcount_table_offset
        header[56..60].copy_from_slice(&1u32.to_be_bytes()); // refcount_table_clusters
        header[60..64].copy_from_slice(&0u32.to_be_bytes()); // nb_snapshots
        header[64..72].copy_from_slice(&0u64.to_be_bytes()); // snapshots_offset
        header[72..80].copy_from_slice(&0u64.to_be_bytes()); // incompatible_features
        header[80..88].copy_from_slice(&0u64.to_be_bytes()); // compatible_features
        header[88..96].copy_from_slice(&0u64.to_be_bytes()); // autoclear_features
        header[96..100].copy_from_slice(&4u32.to_be_bytes()); // refcount_order (2^4 = 16-bit)
        header[100..104].copy_from_slice(&104u32.to_be_bytes()); // header_length

        let mut refcount_table = vec![0u8; QCOW2_CLUSTER_SIZE as usize];
        refcount_table[0..8].copy_from_slice(&(2 * QCOW2_CLUSTER_SIZE).to_be_bytes());

        let mut refcount_block = vec![0u8; QCOW2_CLUSTER_SIZE as usize];
        for idx in 0..total_meta_clusters.min(QCOW2_CLUSTER_SIZE / 2) {
            let pos = (idx as usize) * 2;
            refcount_block[pos..pos + 2].copy_from_slice(&1u16.to_be_bytes());
        }

        let l1_table = vec![0u8; (l1_clusters * QCOW2_CLUSTER_SIZE) as usize];

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        file.write_all(&header)?;
        file.write_all(&refcount_table)?;
        file.write_all(&refcount_block)?;
        file.write_all(&l1_table)?;
        file.sync_all()?;
        Ok(())
    }

    /// Inspects a disk image header (QCOW2 v2/v3 or RAW) and returns integrity diagnostics.
    pub fn inspect_disk(path: &Path) -> Result<DiskInspectionReport, VmError> {
        if !path.is_file() {
            return Ok(DiskInspectionReport {
                path: path.display().to_string(),
                exists: false,
                detected_format: "missing".to_string(),
                virtual_size_bytes: 0,
                actual_disk_bytes: 0,
                qcow2_version: None,
                healthy: false,
                diagnostics: vec!["Disk image file does not exist on storage.".to_string()],
            });
        }

        let meta = fs::metadata(path)?;
        let actual_bytes = meta.len();
        let mut header = [0u8; 104];
        let mut file = File::open(path)?;
        let read_n = file.read(&mut header)?;

        if read_n >= 32 && header[0..4] == QCOW2_MAGIC {
            let version = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
            let cluster_bits = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
            let virtual_size = u64::from_be_bytes([
                header[24], header[25], header[26], header[27], header[28], header[29], header[30],
                header[31],
            ]);
            let mut diags = Vec::new();
            let healthy = (version == 2 || version == 3)
                && (9..=21).contains(&cluster_bits)
                && virtual_size > 0;
            if !healthy {
                diags.push(format!(
                    "Malformed QCOW2 header: version={version}, cluster_bits={cluster_bits}, virtual_size={virtual_size}"
                ));
            } else {
                diags.push(format!(
                    "Valid QCOW2 v{version} image (cluster size {} KiB, virtual {} MiB, on-disk {} KiB).",
                    (1u64 << cluster_bits) / 1024,
                    virtual_size / (1024 * 1024),
                    actual_bytes / 1024
                ));
            }
            return Ok(DiskInspectionReport {
                path: path.display().to_string(),
                exists: true,
                detected_format: "qcow2".to_string(),
                virtual_size_bytes: virtual_size,
                actual_disk_bytes: actual_bytes,
                qcow2_version: Some(version),
                healthy,
                diagnostics: diags,
            });
        }

        Ok(DiskInspectionReport {
            path: path.display().to_string(),
            exists: true,
            detected_format: "raw".to_string(),
            virtual_size_bytes: actual_bytes,
            actual_disk_bytes: actual_bytes,
            qcow2_version: None,
            healthy: actual_bytes > 0,
            diagnostics: vec![format!(
                "RAW image ({} MiB allocated/sparse).",
                actual_bytes / (1024 * 1024)
            )],
        })
    }

    /// Resizes a disk image (growing only to avoid silent filesystem truncation).
    pub fn resize_disk(
        disk: &mut StorageDeviceConfig,
        new_size_mb: u64,
        qemu_img_bin: Option<&Path>,
    ) -> Result<DiskInspectionReport, VmError> {
        if new_size_mb <= disk.size_mb {
            return Err(VmError::Storage(format!(
                "Shrinking disks ({} MB -> {} MB) is blocked to prevent guest filesystem corruption",
                disk.size_mb, new_size_mb
            )));
        }
        let new_bytes = new_size_mb
            .checked_mul(1024 * 1024)
            .ok_or_else(|| VmError::Storage("Disk size overflow".to_string()))?;

        match disk.format {
            DiskFormat::Raw => {
                let file = OpenOptions::new().write(true).open(&disk.path)?;
                file.set_len(new_bytes)?;
                file.sync_all()?;
            }
            DiskFormat::Qcow2 => {
                let bin = qemu_img_bin
                    .filter(|p| p.is_file())
                    .ok_or_else(|| {
                        VmError::Unsupported(
                            "Resizing QCOW2 images requires staged qemu-img binary".to_string(),
                        )
                    })?;
                let status = std::process::Command::new(bin)
                    .args([
                        "resize",
                        &disk.path.to_string_lossy(),
                        &format!("{new_size_mb}M"),
                    ])
                    .status()?;
                if !status.success() {
                    return Err(VmError::Storage(format!(
                        "qemu-img resize failed with status {status}"
                    )));
                }
            }
        }
        disk.size_mb = new_size_mb;
        Self::inspect_disk(&disk.path)
    }

    /// Imports an existing RAW or QCOW2 image safely into the VM directory with optional SHA-256 verification.
    pub fn import_disk_image(
        source: &Path,
        vm_dir: &Path,
        disk_id: &str,
        expected_sha256: Option<&str>,
    ) -> Result<StorageDeviceConfig, VmError> {
        if !source.is_file() {
            return Err(VmError::Storage(format!(
                "Source disk image not found: {}",
                source.display()
            )));
        }
        if disk_id.is_empty()
            || !disk_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(VmError::Security(format!("Invalid disk_id '{disk_id}'")));
        }

        if let Some(expected) = expected_sha256.filter(|s| !s.trim().is_empty()) {
            let actual = Self::compute_sha256(source)?;
            if !actual.eq_ignore_ascii_case(expected.trim()) {
                return Err(VmError::Security(format!(
                    "SHA-256 mismatch for {}: expected {}, got {}",
                    source.display(),
                    expected,
                    actual
                )));
            }
        }

        let report = Self::inspect_disk(source)?;
        if !report.healthy {
            return Err(VmError::Storage(format!(
                "Refusing to import unhealthy disk image: {}",
                report.diagnostics.join("; ")
            )));
        }
        let format = if report.detected_format == "qcow2" {
            DiskFormat::Qcow2
        } else {
            DiskFormat::Raw
        };
        let ext = if format == DiskFormat::Qcow2 {
            "qcow2"
        } else {
            "img"
        };

        fs::create_dir_all(vm_dir)?;
        let dest = vm_dir.join(format!("{disk_id}.{ext}"));
        let part = vm_dir.join(format!("{disk_id}.{ext}.part"));
        let _ = fs::remove_file(&part);
        fs::copy(source, &part)?;
        fs::rename(&part, &dest)?;

        let size_mb = (report.virtual_size_bytes / (1024 * 1024)).max(64);
        Ok(StorageDeviceConfig {
            disk_id: disk_id.to_string(),
            path: dest,
            format,
            bus: DiskBus::VirtioBlk,
            size_mb,
            read_only: false,
            boot_index: None,
        })
    }

    /// Exports a VM disk image safely to a target path and writes a companion `.sha256` checksum file.
    pub fn export_disk_image(source: &Path, dest: &Path) -> Result<String, VmError> {
        if !source.is_file() {
            return Err(VmError::Storage(format!(
                "VM disk not found: {}",
                source.display()
            )));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let part = PathBuf::from(format!("{}.part", dest.display()));
        fs::copy(source, &part)?;
        fs::rename(&part, dest)?;
        let hash = Self::compute_sha256(dest)?;
        let sha_path = PathBuf::from(format!("{}.sha256", dest.display()));
        fs::write(&sha_path, format!("{hash}  {}\n", dest.file_name().unwrap_or_default().to_string_lossy()))?;
        Ok(hash)
    }

    /// Cleans up leftover interrupted `.part` staging files inside a VM directory.
    pub fn cleanup_interrupted_staging(vm_dir: &Path) -> usize {
        let mut removed = 0;
        if let Ok(entries) = fs::read_dir(vm_dir) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e == "part")
                {
                    if fs::remove_file(entry.path()).is_ok() {
                        removed += 1;
                    }
                }
            }
        }
        removed
    }

    /// Computes total on-disk bytes consumed by a VM directory.
    pub fn directory_usage_bytes(vm_dir: &Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = fs::read_dir(vm_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        total = total.saturating_add(meta.len());
                    } else if meta.is_dir() {
                        total = total.saturating_add(Self::directory_usage_bytes(&entry.path()));
                    }
                }
            }
        }
        total
    }

    pub fn compute_sha256(path: &Path) -> Result<String, VmError> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hex::encode(hasher.finalize()))
    }
}
