use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use androidpc_process::DefaultProcessSupervisor;
use androidpc_runtime_api::{ProcessCommand, ProcessId, ProcessSupervisor, Signal};

use crate::config::{
    now_epoch_ms, validate_vm_id, DiskBus, DiskFormat, FirmwareKind, StorageDeviceConfig,
    VmAccelerator, VmArch, VmConfig, VmProfile, VmState,
};
use crate::diagnostics::{
    read_proc_start_time_ticks, KvmCapabilities, QemuEngineCapabilities, VmLogBuffer,
    VmProcessTelemetry,
};
use crate::display_audio::VmDisplayAudioBuilder;
use crate::error::VmError;
use crate::monitor::QmpClient;
use crate::network::VmNetworkBuilder;
use crate::snapshot::{VmSnapshotManager, VmSnapshotMetadata};
use crate::storage::{DiskInspectionReport, VmStorageManager};

/// Legacy + enriched VM summary payload returned by `list_vms`, `create_vm`, `start_vm`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatusInfo {
    pub id: String,
    pub arch: VmArch,
    pub state: VmState,
    pub memory_mb: u32,
    pub cpus: u8,
    pub disk_size_mb: u32,
    pub vnc_port: u16,
    pub qmp_port: u16,
    pub accelerator: String,
    pub pid: Option<ProcessId>,
    pub disk_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmRuntimeSummary {
    pub profile: VmProfile,
    pub effective_accelerator: String,
    pub execution_model_note: String,
    pub pid: Option<ProcessId>,
    pub telemetry: Option<VmProcessTelemetry>,
    pub storage_used_bytes: u64,
    pub snapshot_count: usize,
    pub last_exit_reason: Option<String>,
    pub startup_error: Option<String>,
}

pub struct QemuCommandBuilder;

impl QemuCommandBuilder {
    /// Legacy Phase 1 builder signature preserved for backward compatibility.
    pub fn build_args(config: &VmConfig, kvm_accessible: bool) -> Vec<String> {
        let profile = config.to_profile();
        Self::build_profile_args(&profile, kvm_accessible, Path::new("/nonexistent/firmware"))
    }

    /// Builds hardened, rootless QEMU `11.1.3` CLI arguments from a full Phase 5 `VmProfile`.
    pub fn build_profile_args(
        profile: &VmProfile,
        kvm_accessible: bool,
        firmware_dir: &Path,
    ) -> Vec<String> {
        // KVM is ONLY valid when host is ARM64, guest is ARM64, user enabled KVM, and /dev/kvm is accessible.
        let use_kvm = profile.arch == VmArch::Aarch64
            && profile.accelerator_preference == VmAccelerator::Kvm
            && kvm_accessible;

        let mut args = Vec::new();
        args.extend(["-name".to_string(), profile.id.clone()]);

        // 1. Machine & CPU
        match profile.arch {
            VmArch::Aarch64 => {
                let machine = if profile.machine_type.trim().is_empty() {
                    "virt,gic-version=3"
                } else {
                    profile.machine_type.trim()
                };
                args.extend(["-machine".to_string(), machine.to_string()]);
                let cpu = if use_kvm {
                    "host".to_string()
                } else if profile.cpu.model.trim().is_empty() {
                    "cortex-a72".to_string()
                } else {
                    profile.cpu.model.clone()
                };
                args.extend(["-cpu".to_string(), cpu]);
            }
            VmArch::X86_64 => {
                let machine = if profile.machine_type.trim().is_empty() {
                    "q35"
                } else {
                    profile.machine_type.trim()
                };
                args.extend(["-machine".to_string(), machine.to_string()]);
                let cpu = if profile.cpu.model.trim().is_empty() {
                    "max".to_string()
                } else {
                    profile.cpu.model.clone()
                };
                args.extend(["-cpu".to_string(), cpu]);
            }
        }

        // 2. Accelerator (TCG multi-threaded by default on non-KVM Android)
        if use_kvm {
            args.extend(["-accel".to_string(), "kvm".to_string()]);
        } else {
            let tcg_thread = if profile.accelerator_preference == VmAccelerator::TcgSingle {
                "single"
            } else {
                "multi"
            };
            args.extend([
                "-accel".to_string(),
                format!(
                    "tcg,thread={tcg_thread},tb-size={}",
                    profile.cpu.tcg_tb_size_mb
                ),
            ]);
        }

        // 3. RAM & SMP
        args.extend(["-m".to_string(), format!("{}M", profile.memory_mb)]);
        args.extend([
            "-smp".to_string(),
            format!("cores={},threads={}", profile.cpu.cores, profile.cpu.threads),
        ]);

        // 4. Firmware (EDK2 AAVMF / OVMF / SeaBIOS / Direct Kernel)
        match profile.firmware.kind {
            FirmwareKind::AavmfUefi => {
                let code = profile
                    .firmware
                    .code_path
                    .clone()
                    .unwrap_or_else(|| firmware_dir.join("AAVMF_CODE.fd"));
                if code.is_file() {
                    args.extend([
                        "-drive".to_string(),
                        format!(
                            "if=pflash,format=raw,readonly=on,file={}",
                            code.display()
                        ),
                    ]);
                    if let Some(ref vars) = profile.firmware.vars_path {
                        if vars.is_file() {
                            args.extend([
                                "-drive".to_string(),
                                format!("if=pflash,format=raw,file={}", vars.display()),
                            ]);
                        }
                    }
                }
            }
            FirmwareKind::OvmfUefi => {
                let code = profile
                    .firmware
                    .code_path
                    .clone()
                    .unwrap_or_else(|| firmware_dir.join("OVMF_CODE.fd"));
                if code.is_file() {
                    args.extend([
                        "-drive".to_string(),
                        format!(
                            "if=pflash,format=raw,readonly=on,file={}",
                            code.display()
                        ),
                    ]);
                    if let Some(ref vars) = profile.firmware.vars_path {
                        if vars.is_file() {
                            args.extend([
                                "-drive".to_string(),
                                format!("if=pflash,format=raw,file={}", vars.display()),
                            ]);
                        }
                    }
                }
            }
            FirmwareKind::SeaBios | FirmwareKind::DirectKernel => {}
        }

        // 5. Storage disks (QCOW2 / RAW on VirtIO-Blk, VirtIO-SCSI, SATA, or IDE)
        let mut scsi_added = false;
        for (idx, disk) in profile.disks.iter().enumerate() {
            let ro = if disk.read_only { ",readonly=on" } else { "" };
            let drive_id = format!("hd{idx}");
            match disk.bus {
                DiskBus::VirtioBlk => {
                    args.extend([
                        "-drive".to_string(),
                        format!(
                            "file={},if=virtio,format={},cache=writeback{ro}",
                            disk.path.display(),
                            disk.format.as_str()
                        ),
                    ]);
                }
                DiskBus::VirtioScsi => {
                    if !scsi_added {
                        args.extend([
                            "-device".to_string(),
                            "virtio-scsi-pci,id=scsi0".to_string(),
                        ]);
                        scsi_added = true;
                    }
                    args.extend([
                        "-drive".to_string(),
                        format!(
                            "file={},if=none,id={drive_id},format={},cache=writeback{ro}",
                            disk.path.display(),
                            disk.format.as_str()
                        ),
                        "-device".to_string(),
                        format!("scsi-hd,bus=scsi0.0,drive={drive_id}"),
                    ]);
                }
                DiskBus::Sata | DiskBus::Ide => {
                    let iface = if disk.bus == DiskBus::Sata { "none" } else { "ide" };
                    args.extend([
                        "-drive".to_string(),
                        format!(
                            "file={},if={iface},id={drive_id},format={}{ro}",
                            disk.path.display(),
                            disk.format.as_str()
                        ),
                    ]);
                    if disk.bus == DiskBus::Sata {
                        args.extend([
                            "-device".to_string(),
                            format!("ide-hd,drive={drive_id}"),
                        ]);
                    }
                }
            }
        }

        // 6. Boot media (ISO / Direct Kernel)
        if let Some(ref iso) = profile.boot.iso_path {
            if profile.arch == VmArch::Aarch64 {
                if !scsi_added {
                    args.extend([
                        "-device".to_string(),
                        "virtio-scsi-pci,id=scsi0".to_string(),
                    ]);
                }
                args.extend([
                    "-drive".to_string(),
                    format!("file={},if=none,id=cd0,media=cdrom,readonly=on", iso.display()),
                    "-device".to_string(),
                    "scsi-cd,bus=scsi0.0,drive=cd0".to_string(),
                ]);
            } else {
                args.extend(["-cdrom".to_string(), iso.display().to_string()]);
                args.extend(["-boot".to_string(), "d".to_string()]);
            }
        }
        if let Some(ref kernel) = profile.boot.direct_kernel {
            args.extend(["-kernel".to_string(), kernel.display().to_string()]);
            if let Some(ref initrd) = profile.boot.direct_initrd {
                args.extend(["-initrd".to_string(), initrd.display().to_string()]);
            }
            if let Some(ref cmdline) = profile.boot.kernel_cmdline {
                if !cmdline.trim().is_empty() {
                    args.extend(["-append".to_string(), cmdline.clone()]);
                }
            }
        }

        // 7. Rootless SLIRP networking
        args.extend(VmNetworkBuilder::build_args(&profile.network));

        // 8. Display & USB Tablet/Keyboard input
        args.extend(VmDisplayAudioBuilder::build_display_and_input_args(
            profile.arch,
            &profile.display,
        ));

        // 9. Modular Audio
        args.extend(VmDisplayAudioBuilder::build_audio_args(&profile.audio));

        // 10. QMP local TCP management interface
        args.extend([
            "-qmp".to_string(),
            format!("tcp:127.0.0.1:{},server,nowait", profile.qmp_port),
        ]);

        // 11. Safe defaults: no default host-folder sharing, no auto-reboot loop
        args.push("-nodefaults".to_string());
        args.push("-no-reboot".to_string());
        args
    }
}

struct ManagedVm {
    profile: VmProfile,
    pid: Option<ProcessId>,
    pid_start_ticks: Option<u64>,
    last_exit_reason: Option<String>,
    startup_error: Option<String>,
    logs: VmLogBuffer,
}

pub struct QemuSupervisor {
    vms_dir: PathBuf,
    qemu_bin_dir: PathBuf,
    firmware_dir: PathBuf,
    supervisor: DefaultProcessSupervisor,
    vms: RwLock<HashMap<String, ManagedVm>>,
}

impl QemuSupervisor {
    pub fn new(base_data_dir: &Path, qemu_bin_dir: &Path) -> Self {
        let vms_dir = base_data_dir.join("vms");
        let firmware_dir = qemu_bin_dir
            .parent()
            .unwrap_or(qemu_bin_dir)
            .join("qemu-firmware");
        let _ = fs::create_dir_all(&vms_dir);
        let mgr = Self {
            vms_dir,
            qemu_bin_dir: qemu_bin_dir.to_path_buf(),
            firmware_dir,
            supervisor: DefaultProcessSupervisor::new(),
            vms: RwLock::new(HashMap::new()),
        };
        mgr.reload_persisted();
        mgr
    }

    pub fn capabilities(&self) -> QemuEngineCapabilities {
        QemuEngineCapabilities::probe(&self.qemu_bin_dir, &self.firmware_dir)
    }

    fn reload_persisted(&self) {
        let Ok(entries) = fs::read_dir(&self.vms_dir) else {
            return;
        };
        let Ok(mut guard) = self.vms.write() else {
            return;
        };
        for entry in entries.flatten() {
            let vm_dir = entry.path();
            if !vm_dir.is_dir() {
                continue;
            }
            let _ = VmStorageManager::cleanup_interrupted_staging(&vm_dir);

            let profile_path = vm_dir.join("vm_profile.json");
            let legacy_path = vm_dir.join("vm.json");

            let loaded = if let Ok(raw) = fs::read_to_string(&profile_path) {
                serde_json::from_str::<VmProfile>(&raw).ok()
            } else if let Ok(raw) = fs::read_to_string(&legacy_path) {
                serde_json::from_str::<VmConfig>(&raw)
                    .ok()
                    .map(|c| c.to_profile())
            } else {
                None
            };

            if let Some(mut profile) = loaded {
                profile.state = VmState::Stopped;
                let mut logs = VmLogBuffer::default();
                logs.push(format!(
                    "Loaded persisted VM '{}' ({}, QEMU {}).",
                    profile.id,
                    profile.arch.as_str(),
                    profile.qemu_version
                ));
                guard.insert(
                    profile.id.clone(),
                    ManagedVm {
                        profile,
                        pid: None,
                        pid_start_ticks: None,
                        last_exit_reason: None,
                        startup_error: None,
                        logs,
                    },
                );
            }
        }
    }

    fn persist_profile(&self, profile: &VmProfile) -> Result<(), VmError> {
        let vm_dir = self.vms_dir.join(&profile.id);
        fs::create_dir_all(&vm_dir)?;
        let part = vm_dir.join("vm_profile.json.part");
        let dest = vm_dir.join("vm_profile.json");
        fs::write(&part, serde_json::to_string_pretty(profile)?)?;
        fs::rename(&part, &dest)?;
        Ok(())
    }

    fn qemu_img_path(&self) -> PathBuf {
        self.qemu_bin_dir.join("qemu-img")
    }

    /// Legacy Phase 1 creation entrypoint preserved intact.
    pub fn create_vm(
        &self,
        id: &str,
        arch: VmArch,
        memory_mb: u32,
        cpus: u8,
        disk_size_mb: u32,
    ) -> Result<VmStatusInfo, VmError> {
        let summary = self.create_vm_with_options(
            id,
            id,
            arch,
            memory_mb,
            cpus,
            disk_size_mb as u64,
            DiskFormat::Qcow2,
            None,
        )?;
        Ok(Self::to_legacy_status(&summary.profile, summary.pid))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_vm_with_options(
        &self,
        id: &str,
        name: &str,
        arch: VmArch,
        memory_mb: u32,
        cpus: u8,
        disk_size_mb: u64,
        disk_format: DiskFormat,
        iso_path: Option<PathBuf>,
    ) -> Result<VmRuntimeSummary, VmError> {
        validate_vm_id(id)?;
        let (vnc_display, qmp_port) = {
            let guard = self
                .vms
                .read()
                .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
            if guard.contains_key(id) {
                return Err(VmError::AlreadyExists(id.to_string()));
            }
            let used_vnc: Vec<u16> = guard.values().map(|v| v.profile.display.vnc_display).collect();
            let used_qmp: Vec<u16> = guard.values().map(|v| v.profile.qmp_port).collect();
            let vnc = (10u16..200).find(|p| !used_vnc.contains(p)).unwrap_or(10);
            let qmp = (4444u16..4644).find(|p| !used_qmp.contains(p)).unwrap_or(4444);
            (vnc, qmp)
        };

        let vm_dir = self.vms_dir.join(id);
        let mut profile = VmProfile::default_preset(
            id,
            name,
            arch,
            memory_mb,
            cpus,
            disk_size_mb,
            disk_format,
            &vm_dir,
            vnc_display,
            qmp_port,
        );
        profile.boot.iso_path = iso_path;
        profile.validate()?;

        fs::create_dir_all(&vm_dir)?;
        let primary = &profile.disks[0];
        let qemu_img = self.qemu_img_path();
        VmStorageManager::create_disk(
            &primary.path,
            primary.format,
            primary.size_mb,
            Some(&qemu_img),
        )?;

        self.persist_profile(&profile)?;

        let mut logs = VmLogBuffer::default();
        logs.push(format!(
            "Created VM '{}' ({}, {} MB RAM, {} vCPU, {} {} MB disk).",
            profile.id,
            profile.arch.as_str(),
            profile.memory_mb,
            profile.cpu.cores,
            primary.format.as_str(),
            primary.size_mb
        ));

        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        guard.insert(
            id.to_string(),
            ManagedVm {
                profile: profile.clone(),
                pid: None,
                pid_start_ticks: None,
                last_exit_reason: None,
                startup_error: None,
                logs,
            },
        );
        info!("Created Phase 5 QEMU VM '{id}' ({:?})", arch);
        Ok(self.build_summary_from_entry(guard.get(id).unwrap()))
    }

    pub fn update_vm_profile(&self, updated: VmProfile) -> Result<VmRuntimeSummary, VmError> {
        updated.validate()?;
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let entry = guard
            .get_mut(&updated.id)
            .ok_or_else(|| VmError::NotFound(updated.id.clone()))?;
        if matches!(entry.profile.state, VmState::Running | VmState::Paused | VmState::Starting) {
            return Err(VmError::AlreadyRunning(updated.id.clone()));
        }
        entry.profile = updated;
        self.persist_profile(&entry.profile)?;
        entry.logs.push("Updated VM hardware configuration.".to_string());
        Ok(self.build_summary_from_entry(entry))
    }

    pub fn clone_vm(&self, source_id: &str, new_id: &str, new_name: &str) -> Result<VmRuntimeSummary, VmError> {
        validate_vm_id(new_id)?;
        let (src_profile, vnc_display, qmp_port) = {
            let guard = self
                .vms
                .read()
                .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
            if guard.contains_key(new_id) {
                return Err(VmError::AlreadyExists(new_id.to_string()));
            }
            let src = guard
                .get(source_id)
                .ok_or_else(|| VmError::NotFound(source_id.to_string()))?;
            if matches!(src.profile.state, VmState::Running | VmState::Paused) {
                return Err(VmError::AlreadyRunning(source_id.to_string()));
            }
            let used_vnc: Vec<u16> = guard.values().map(|v| v.profile.display.vnc_display).collect();
            let used_qmp: Vec<u16> = guard.values().map(|v| v.profile.qmp_port).collect();
            let vnc = (10u16..200).find(|p| !used_vnc.contains(p)).unwrap_or(10);
            let qmp = (4444u16..4644).find(|p| !used_qmp.contains(p)).unwrap_or(4444);
            (src.profile.clone(), vnc, qmp)
        };

        let dst_dir = self.vms_dir.join(new_id);
        fs::create_dir_all(&dst_dir)?;
        let mut cloned = src_profile;
        cloned.id = new_id.to_string();
        cloned.name = if new_name.trim().is_empty() {
            format!("{} (Clone)", cloned.name)
        } else {
            new_name.trim().to_string()
        };
        cloned.display.vnc_display = vnc_display;
        cloned.qmp_port = qmp_port;
        cloned.created_at_epoch_ms = now_epoch_ms();
        cloned.last_used_at_epoch_ms = None;
        cloned.state = VmState::Stopped;

        for disk in &mut cloned.disks {
            let file_name = disk
                .path
                .file_name()
                .map(|f| f.to_os_string())
                .unwrap_or_else(|| format!("{}.img", disk.disk_id).into());
            let new_disk_path = dst_dir.join(file_name);
            if disk.path.is_file() {
                let part = PathBuf::from(format!("{}.part", new_disk_path.display()));
                fs::copy(&disk.path, &part)?;
                fs::rename(&part, &new_disk_path)?;
            }
            disk.path = new_disk_path;
        }

        self.persist_profile(&cloned)?;
        let mut logs = VmLogBuffer::default();
        logs.push(format!("Cloned VM '{new_id}' from '{source_id}'."));

        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        guard.insert(
            new_id.to_string(),
            ManagedVm {
                profile: cloned,
                pid: None,
                pid_start_ticks: None,
                last_exit_reason: None,
                startup_error: None,
                logs,
            },
        );
        Ok(self.build_summary_from_entry(guard.get(new_id).unwrap()))
    }

    pub fn start_vm(&self, id: &str) -> Result<VmStatusInfo, VmError> {
        let summary = self.start_vm_detailed(id)?;
        Ok(Self::to_legacy_status(&summary.profile, summary.pid))
    }

    pub fn start_vm_detailed(&self, id: &str) -> Result<VmRuntimeSummary, VmError> {
        let kvm = KvmCapabilities::probe();
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(id)
            .ok_or_else(|| VmError::NotFound(id.to_string()))?;

        if let Some(pid) = vm.pid {
            if self.is_owned_process_alive(pid, vm.pid_start_ticks) {
                return Err(VmError::AlreadyRunning(id.to_string()));
            }
        }

        let bin_name = vm.profile.arch.qemu_binary();
        let qemu_bin = self.qemu_bin_dir.join(bin_name);
        let program = if qemu_bin.is_file() {
            qemu_bin.to_string_lossy().to_string()
        } else {
            bin_name.to_string()
        };

        let args = QemuCommandBuilder::build_profile_args(
            &vm.profile,
            kvm.accessible,
            &self.firmware_dir,
        );
        let cmd = ProcessCommand {
            program: program.clone(),
            args: args.clone(),
            env: Vec::new(),
            cwd: Some(self.vms_dir.join(id).to_string_lossy().to_string()),
            uid: None,
            gid: None,
        };

        vm.profile.state = VmState::Starting;
        vm.startup_error = None;
        vm.logs.push(format!("Launching {program} {}", args.join(" ")));

        match self.supervisor.spawn_process(&cmd) {
            Ok(pid) => {
                vm.pid = Some(pid);
                vm.pid_start_ticks = read_proc_start_time_ticks(pid);
                vm.profile.state = VmState::Running;
                vm.profile.last_used_at_epoch_ms = Some(now_epoch_ms());
                let _ = self.persist_profile(&vm.profile);
                vm.logs.push(format!("QEMU started cleanly (PID {pid})."));
                info!("Started QEMU VM '{id}' with PID {pid}");
                Ok(self.build_summary_from_entry(vm))
            }
            Err(err) => {
                warn!("Failed to spawn QEMU binary for '{id}': {err}");
                vm.profile.state = VmState::Error;
                vm.startup_error = Some(err.to_string());
                vm.logs.push(format!("Startup failed: {err}"));
                Err(VmError::Runtime(err))
            }
        }
    }

    pub fn pause_vm(&self, id: &str) -> Result<VmRuntimeSummary, VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(id)
            .ok_or_else(|| VmError::NotFound(id.to_string()))?;
        if vm.profile.state != VmState::Running {
            return Err(VmError::NotRunning(id.to_string()));
        }
        QmpClient::pause(vm.profile.qmp_port)?;
        vm.profile.state = VmState::Paused;
        vm.logs.push("VM paused via QMP 'stop'.".to_string());
        Ok(self.build_summary_from_entry(vm))
    }

    pub fn resume_vm(&self, id: &str) -> Result<VmRuntimeSummary, VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(id)
            .ok_or_else(|| VmError::NotFound(id.to_string()))?;
        if vm.profile.state != VmState::Paused {
            return Err(VmError::InvalidConfig(format!("VM '{id}' is not paused")));
        }
        QmpClient::resume(vm.profile.qmp_port)?;
        vm.profile.state = VmState::Running;
        vm.logs.push("VM resumed via QMP 'cont'.".to_string());
        Ok(self.build_summary_from_entry(vm))
    }

    pub fn restart_vm(&self, id: &str) -> Result<VmRuntimeSummary, VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(id)
            .ok_or_else(|| VmError::NotFound(id.to_string()))?;
        if !matches!(vm.profile.state, VmState::Running | VmState::Paused) {
            return Err(VmError::NotRunning(id.to_string()));
        }
        QmpClient::system_reset(vm.profile.qmp_port)?;
        vm.profile.state = VmState::Running;
        vm.logs.push("VM reset triggered via QMP 'system_reset'.".to_string());
        Ok(self.build_summary_from_entry(vm))
    }

    /// Graceful ACPI shutdown via QMP `system_powerdown`, escalating cleanly with bounded timeout
    /// and strict PID start-time verification (preventing PID-reuse accidents).
    pub fn graceful_shutdown_vm(&self, id: &str, timeout_ms: u64) -> Result<(), VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(id)
            .ok_or_else(|| VmError::NotFound(id.to_string()))?;

        vm.profile.state = VmState::Stopping;
        let _ = QmpClient::system_powerdown(vm.profile.qmp_port);
        vm.logs.push("Sent ACPI graceful shutdown via QMP 'system_powerdown'.".to_string());

        if let Some(pid) = vm.pid {
            let deadline = Instant::now() + Duration::from_millis(timeout_ms.clamp(200, 15_000));
            while Instant::now() < deadline {
                if !self.is_owned_process_alive(pid, vm.pid_start_ticks) {
                    vm.pid = None;
                    vm.pid_start_ticks = None;
                    vm.profile.state = VmState::Stopped;
                    vm.last_exit_reason = Some("graceful_acpi_shutdown".to_string());
                    vm.logs.push("VM exited cleanly after ACPI shutdown.".to_string());
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(50));
            }

            // Bounded escalation: QMP quit -> SIGTERM -> SIGKILL only if PID identity matches
            let _ = QmpClient::quit(vm.profile.qmp_port);
            if self.is_owned_process_alive(pid, vm.pid_start_ticks) {
                let _ = self.supervisor.send_signal(pid, Signal::Term);
                std::thread::sleep(Duration::from_millis(150));
            }
            if self.is_owned_process_alive(pid, vm.pid_start_ticks) {
                let _ = self.supervisor.send_signal(pid, Signal::Kill);
                vm.last_exit_reason = Some("escalated_sigkill".to_string());
                vm.logs.push("Graceful shutdown timed out; escalated cleanly to SIGKILL on verified PID.".to_string());
            } else {
                vm.last_exit_reason = Some("sigterm_shutdown".to_string());
            }
            vm.pid = None;
            vm.pid_start_ticks = None;
        }
        vm.profile.state = VmState::Stopped;
        Ok(())
    }

    pub fn force_stop_vm(&self, id: &str) -> Result<(), VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(id)
            .ok_or_else(|| VmError::NotFound(id.to_string()))?;

        if let Some(pid) = vm.pid.take() {
            if self.is_owned_process_alive(pid, vm.pid_start_ticks) {
                let _ = self.supervisor.send_signal(pid, Signal::Kill);
            }
            vm.pid_start_ticks = None;
        }
        vm.profile.state = VmState::Stopped;
        vm.last_exit_reason = Some("force_stopped".to_string());
        vm.logs.push("VM force stopped via verified-PID SIGKILL.".to_string());
        Ok(())
    }

    pub fn stop_vm(&self, id: &str) -> Result<(), VmError> {
        self.graceful_shutdown_vm(id, 1500)
    }

    pub fn delete_vm(&self, id: &str) -> Result<(), VmError> {
        validate_vm_id(id)?;
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        if let Some(mut vm) = guard.remove(id) {
            if let Some(pid) = vm.pid.take() {
                if self.is_owned_process_alive(pid, vm.pid_start_ticks) {
                    let _ = self.supervisor.send_signal(pid, Signal::Kill);
                }
            }
        }
        let vm_dir = self.vms_dir.join(id);
        if vm_dir.exists() {
            fs::remove_dir_all(vm_dir)?;
        }
        info!("Deleted QEMU VM '{id}'");
        Ok(())
    }

    pub fn attach_disk(
        &self,
        vm_id: &str,
        disk_id: &str,
        format: DiskFormat,
        size_mb: u64,
        bus: DiskBus,
    ) -> Result<VmRuntimeSummary, VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        if matches!(vm.profile.state, VmState::Running | VmState::Paused) {
            return Err(VmError::AlreadyRunning(vm_id.to_string()));
        }
        if vm.profile.disks.iter().any(|d| d.disk_id == disk_id) {
            return Err(VmError::Storage(format!("Disk '{disk_id}' already attached")));
        }

        let ext = if format == DiskFormat::Qcow2 { "qcow2" } else { "img" };
        let path = self.vms_dir.join(vm_id).join(format!("{disk_id}.{ext}"));
        let qemu_img = self.qemu_img_path();
        VmStorageManager::create_disk(&path, format, size_mb, Some(&qemu_img))?;

        vm.profile.disks.push(StorageDeviceConfig {
            disk_id: disk_id.to_string(),
            path,
            format,
            bus,
            size_mb,
            read_only: false,
            boot_index: None,
        });
        self.persist_profile(&vm.profile)?;
        vm.logs.push(format!(
            "Attached disk '{disk_id}' ({}, {size_mb} MB).",
            format.as_str()
        ));
        Ok(self.build_summary_from_entry(vm))
    }

    pub fn detach_disk(&self, vm_id: &str, disk_id: &str, delete_file: bool) -> Result<VmRuntimeSummary, VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        if matches!(vm.profile.state, VmState::Running | VmState::Paused) {
            return Err(VmError::AlreadyRunning(vm_id.to_string()));
        }
        if vm.profile.disks.len() <= 1 {
            return Err(VmError::Storage(
                "Cannot detach the primary/only disk from a VM".to_string(),
            ));
        }
        let pos = vm
            .profile
            .disks
            .iter()
            .position(|d| d.disk_id == disk_id)
            .ok_or_else(|| VmError::Storage(format!("Disk '{disk_id}' not found")))?;
        let removed = vm.profile.disks.remove(pos);
        if delete_file && removed.path.is_file() {
            let _ = fs::remove_file(&removed.path);
        }
        self.persist_profile(&vm.profile)?;
        vm.logs.push(format!("Detached disk '{disk_id}'."));
        Ok(self.build_summary_from_entry(vm))
    }

    pub fn resize_disk(&self, vm_id: &str, disk_id: &str, new_size_mb: u64) -> Result<DiskInspectionReport, VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        if matches!(vm.profile.state, VmState::Running | VmState::Paused) {
            return Err(VmError::AlreadyRunning(vm_id.to_string()));
        }
        let disk = vm
            .profile
            .disks
            .iter_mut()
            .find(|d| d.disk_id == disk_id)
            .ok_or_else(|| VmError::Storage(format!("Disk '{disk_id}' not found")))?;
        let qemu_img = self.qemu_img_path();
        let report = VmStorageManager::resize_disk(disk, new_size_mb, Some(&qemu_img))?;
        self.persist_profile(&vm.profile)?;
        vm.logs.push(format!("Resized disk '{disk_id}' to {new_size_mb} MB."));
        Ok(report)
    }

    pub fn import_disk_image(
        &self,
        vm_id: &str,
        source_path: &Path,
        disk_id: &str,
        expected_sha256: Option<&str>,
    ) -> Result<VmRuntimeSummary, VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        if matches!(vm.profile.state, VmState::Running | VmState::Paused) {
            return Err(VmError::AlreadyRunning(vm_id.to_string()));
        }
        let imported = VmStorageManager::import_disk_image(
            source_path,
            &self.vms_dir.join(vm_id),
            disk_id,
            expected_sha256,
        )?;
        vm.profile.disks.retain(|d| d.disk_id != disk_id);
        vm.profile.disks.push(imported);
        self.persist_profile(&vm.profile)?;
        vm.logs.push(format!(
            "Imported verified disk '{disk_id}' from {}.",
            source_path.display()
        ));
        Ok(self.build_summary_from_entry(vm))
    }

    pub fn inspect_vm_disks(&self, vm_id: &str) -> Result<Vec<DiskInspectionReport>, VmError> {
        let guard = self
            .vms
            .read()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        let mut reports = Vec::new();
        for disk in &vm.profile.disks {
            reports.push(VmStorageManager::inspect_disk(&disk.path)?);
        }
        Ok(reports)
    }

    pub fn list_snapshots(&self, vm_id: &str) -> Result<Vec<VmSnapshotMetadata>, VmError> {
        validate_vm_id(vm_id)?;
        Ok(VmSnapshotManager::list_snapshots(&self.vms_dir.join(vm_id)))
    }

    pub fn create_snapshot(
        &self,
        vm_id: &str,
        snapshot_id: &str,
        name: &str,
        description: &str,
    ) -> Result<VmSnapshotMetadata, VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        let is_running = matches!(vm.profile.state, VmState::Running | VmState::Paused);
        let qemu_img = self.qemu_img_path();
        let meta = VmSnapshotManager::create_snapshot(
            &self.vms_dir.join(vm_id),
            &vm.profile,
            snapshot_id,
            name,
            description,
            is_running,
            Some(&qemu_img),
        )?;
        vm.logs.push(format!(
            "Created snapshot '{}' ({}).",
            meta.snapshot_id, meta.mechanism
        ));
        Ok(meta)
    }

    pub fn restore_snapshot(&self, vm_id: &str, snapshot_id: &str) -> Result<(), VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        let is_running = matches!(vm.profile.state, VmState::Running | VmState::Paused);
        let qemu_img = self.qemu_img_path();
        VmSnapshotManager::restore_snapshot(
            &self.vms_dir.join(vm_id),
            &vm.profile,
            snapshot_id,
            is_running,
            Some(&qemu_img),
        )?;
        vm.logs.push(format!("Restored snapshot '{snapshot_id}'."));
        Ok(())
    }

    pub fn delete_snapshot(&self, vm_id: &str, snapshot_id: &str) -> Result<(), VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        let is_running = matches!(vm.profile.state, VmState::Running | VmState::Paused);
        let qemu_img = self.qemu_img_path();
        VmSnapshotManager::delete_snapshot(
            &self.vms_dir.join(vm_id),
            &vm.profile,
            snapshot_id,
            is_running,
            Some(&qemu_img),
        )?;
        vm.logs.push(format!("Deleted snapshot '{snapshot_id}'."));
        Ok(())
    }

    pub fn get_vm_logs(&self, vm_id: &str) -> Result<Vec<String>, VmError> {
        let guard = self
            .vms
            .read()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get(vm_id)
            .ok_or_else(|| VmError::NotFound(vm_id.to_string()))?;
        Ok(vm.logs.snapshot())
    }

    pub fn list_vm_summaries(&self) -> Vec<VmRuntimeSummary> {
        let Ok(mut guard) = self.vms.write() else {
            return Vec::new();
        };
        let mut summaries = Vec::new();
        for vm in guard.values_mut() {
            if matches!(vm.profile.state, VmState::Running | VmState::Paused) {
                if let Some(pid) = vm.pid {
                    if !self.is_owned_process_alive(pid, vm.pid_start_ticks) {
                        vm.profile.state = VmState::Stopped;
                        vm.pid = None;
                        vm.pid_start_ticks = None;
                        vm.last_exit_reason = Some("process_exited".to_string());
                    }
                }
            }
            summaries.push(self.build_summary_from_entry(vm));
        }
        summaries.sort_by(|a, b| a.profile.id.cmp(&b.profile.id));
        summaries
    }

    pub fn list_vms(&self) -> Vec<VmStatusInfo> {
        self.list_vm_summaries()
            .into_iter()
            .map(|s| Self::to_legacy_status(&s.profile, s.pid))
            .collect()
    }

    fn is_owned_process_alive(&self, pid: ProcessId, expected_start_ticks: Option<u64>) -> bool {
        if !self.supervisor.is_alive(pid) {
            return false;
        }
        match (expected_start_ticks, read_proc_start_time_ticks(pid)) {
            (Some(exp), Some(actual)) => exp == actual,
            _ => true,
        }
    }

    fn build_summary_from_entry(&self, vm: &ManagedVm) -> VmRuntimeSummary {
        let kvm = KvmCapabilities::probe();
        let use_kvm = vm.profile.arch == VmArch::Aarch64
            && vm.profile.accelerator_preference == VmAccelerator::Kvm
            && kvm.accessible;
        let effective_accelerator = if use_kvm {
            "kvm".to_string()
        } else {
            vm.profile.accelerator_preference.as_label().to_string()
        };
        let telemetry = vm
            .pid
            .map(|pid| VmProcessTelemetry::inspect(pid, vm.pid_start_ticks));
        let vm_dir = self.vms_dir.join(&vm.profile.id);
        let storage_used_bytes = VmStorageManager::directory_usage_bytes(&vm_dir);
        let snapshot_count = VmSnapshotManager::list_snapshots(&vm_dir).len();

        VmRuntimeSummary {
            profile: vm.profile.clone(),
            effective_accelerator,
            execution_model_note: vm.profile.arch.execution_model_note(use_kvm),
            pid: vm.pid,
            telemetry,
            storage_used_bytes,
            snapshot_count,
            last_exit_reason: vm.last_exit_reason.clone(),
            startup_error: vm.startup_error.clone(),
        }
    }

    fn to_legacy_status(profile: &VmProfile, pid: Option<ProcessId>) -> VmStatusInfo {
        let kvm = KvmCapabilities::probe();
        let accel = if profile.arch == VmArch::Aarch64
            && profile.accelerator_preference == VmAccelerator::Kvm
            && kvm.accessible
        {
            "kvm".to_string()
        } else {
            "tcg".to_string()
        };
        let primary = profile.primary_disk();
        VmStatusInfo {
            id: profile.id.clone(),
            arch: profile.arch,
            state: profile.state,
            memory_mb: profile.memory_mb,
            cpus: profile.cpu.cores,
            disk_size_mb: primary.map(|d| d.size_mb as u32).unwrap_or(0),
            vnc_port: profile.vnc_port(),
            qmp_port: profile.qmp_port,
            accelerator: accel,
            pid,
            disk_exists: primary.is_some_and(|d| d.path.exists()),
        }
    }
}
