pub mod config;
pub mod diagnostics;
pub mod display_audio;
pub mod error;
pub mod images;
pub mod monitor;
pub mod network;
pub mod qemu_engine;
pub mod snapshot;
pub mod storage;

pub use config::{
    AudioVmConfig, BootConfig, CpuConfig, DiskBus, DiskFormat, DisplayVmConfig, FirmwareConfig,
    FirmwareKind, NetworkConfig, NetworkMode, PortForwardRule, StorageDeviceConfig, VmAccelerator,
    VmArch, VmConfig, VmDisplayBackend, VmProfile, VmState, VmAudioBackend, EDK2_PINNED_VERSION,
    LIBSLIRP_PINNED_VERSION, QEMU_PINNED_VERSION,
};
pub use diagnostics::{
    AvfCapabilities, KvmCapabilities, QemuEngineCapabilities, VmLogBuffer, VmProcessTelemetry,
};
pub use display_audio::VmDisplayAudioBuilder;
pub use error::VmError;
pub use images::{VmGuestImageEntry, VmImageCatalog};
pub use monitor::QmpClient;
pub use network::VmNetworkBuilder;
pub use qemu_engine::{QemuCommandBuilder, QemuSupervisor, VmRuntimeSummary, VmStatusInfo};
pub use snapshot::{VmSnapshotManager, VmSnapshotMetadata};
pub use storage::{DiskInspectionReport, VmStorageManager, QCOW2_MAGIC};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_qemu_command_builder_aarch64_tcg_and_kvm() {
        let cfg = VmConfig {
            id: "alpine-arm64".to_string(),
            arch: VmArch::Aarch64,
            memory_mb: 1024,
            cpus: 4,
            disk_size_mb: 2048,
            disk_image_path: PathBuf::from("/data/vms/alpine/disk.img"),
            iso_path: None,
            vnc_display: 10,
            qmp_port: 4444,
            enable_kvm: true,
        };
        cfg.validate().expect("valid config");

        let tcg_args = QemuCommandBuilder::build_args(&cfg, false);
        assert!(tcg_args.iter().any(|a| a.starts_with("virt")));
        assert!(tcg_args.iter().any(|a| a.starts_with("tcg,thread=multi")));
        assert!(tcg_args.contains(&"vnc=127.0.0.1:10".to_string()));
        assert!(tcg_args.contains(&"usb-tablet,bus=xhci.0".to_string()));

        let kvm_args = QemuCommandBuilder::build_args(&cfg, true);
        assert!(kvm_args.contains(&"host".to_string()));
        assert!(kvm_args.contains(&"kvm".to_string()));
    }

    #[test]
    fn test_qemu_command_builder_x86_64_never_uses_kvm_on_arm64() {
        let cfg = VmConfig {
            id: "debian-x64".to_string(),
            arch: VmArch::X86_64,
            memory_mb: 2048,
            cpus: 2,
            disk_size_mb: 4096,
            disk_image_path: PathBuf::from("/data/vms/debian/disk.qcow2"),
            iso_path: Some(PathBuf::from("/data/isos/boot.iso")),
            vnc_display: 11,
            qmp_port: 4445,
            enable_kvm: true,
        };
        let args = QemuCommandBuilder::build_args(&cfg, true);
        assert!(args.contains(&"q35".to_string()));
        assert!(args.contains(&"max".to_string()));
        assert!(args.iter().any(|a| a.starts_with("tcg,thread=multi")));
        assert!(!args.contains(&"kvm".to_string()));
        assert!(args.contains(&"-cdrom".to_string()));
        assert_eq!(cfg.vnc_port(), 5911);
    }

    #[test]
    fn test_qcow2_creation_inspection_and_vm_lifecycle() {
        let tmp = std::env::temp_dir().join("androidpc_phase5_vm_test");
        let _ = std::fs::remove_dir_all(&tmp);
        let sup = QemuSupervisor::new(&tmp, Path::new("/nonexistent/bin"));

        let created = sup
            .create_vm("test-vm", VmArch::Aarch64, 512, 2, 512)
            .expect("create vm");
        assert_eq!(created.id, "test-vm");
        assert_eq!(created.vnc_port, 5910);
        assert!(created.disk_exists);

        let inspections = sup.inspect_vm_disks("test-vm").expect("inspect");
        assert_eq!(inspections.len(), 1);
        assert_eq!(inspections[0].detected_format, "qcow2");
        assert_eq!(inspections[0].qcow2_version, Some(3));
        assert!(inspections[0].healthy);

        // Attach secondary RAW disk & resize
        let updated = sup
            .attach_disk("test-vm", "data1", DiskFormat::Raw, 128, DiskBus::VirtioBlk)
            .expect("attach secondary disk");
        assert_eq!(updated.profile.disks.len(), 2);

        let resized = sup.resize_disk("test-vm", "data1", 256).expect("resize raw");
        assert_eq!(resized.virtual_size_bytes, 256 * 1024 * 1024);

        // Offline verified snapshot & restore
        let snap = sup
            .create_snapshot("test-vm", "clean-base", "Clean Base", "Initial disk state")
            .expect("snapshot");
        assert_eq!(snap.mechanism, "offline_verified_copy");
        assert!(snap.disk_sha256.is_some());
        sup.restore_snapshot("test-vm", "clean-base").expect("restore");
        sup.delete_snapshot("test-vm", "clean-base").expect("delete snap");

        // Clone VM
        let cloned = sup
            .clone_vm("test-vm", "test-vm-clone", "Cloned VM")
            .expect("clone");
        assert_eq!(cloned.profile.id, "test-vm-clone");
        assert_ne!(cloned.profile.vnc_port(), created.vnc_port);

        // Detach secondary disk and delete both VMs
        sup.detach_disk("test-vm", "data1", true).expect("detach");
        sup.delete_vm("test-vm").expect("delete vm");
        sup.delete_vm("test-vm-clone").expect("delete clone");
        assert!(sup.list_vms().is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_official_image_catalog_and_qmp_framing() {
        let images = VmImageCatalog::list_official_images();
        assert!(images.iter().any(|i| i.arch == VmArch::Aarch64));
        assert!(images.iter().any(|i| i.arch == VmArch::X86_64));
        let cmd = QmpClient::format_command("system_powerdown", None);
        assert!(cmd.contains("system_powerdown"));
    }
}
