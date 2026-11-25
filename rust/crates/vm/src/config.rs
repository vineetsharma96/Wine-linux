use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::VmError;

pub const QEMU_PINNED_VERSION: &str = "11.1.3";
pub const EDK2_PINNED_VERSION: &str = "edk2-stable202608";
pub const LIBSLIRP_PINNED_VERSION: &str = "4.9.7";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VmArch {
    Aarch64,
    X86_64,
}

impl VmArch {
    pub fn parse(s: &str) -> Result<Self, VmError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "aarch64" | "arm64" => Ok(Self::Aarch64),
            "x86_64" | "amd64" | "x64" => Ok(Self::X86_64),
            other => Err(VmError::InvalidConfig(format!(
                "Unsupported VM architecture: {other}"
            ))),
        }
    }

    pub fn qemu_binary(&self) -> &'static str {
        match self {
            Self::Aarch64 => "qemu-system-aarch64",
            Self::X86_64 => "qemu-system-x86_64",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::X86_64 => "x86_64",
        }
    }

    /// Explicit honesty note distinguishing emulation from hardware virtualization.
    pub fn execution_model_note(&self, kvm_accessible: bool) -> String {
        match (self, kvm_accessible) {
            (Self::Aarch64, true) => {
                "ARM64 guest on ARM64 host with /dev/kvm hardware virtualization available."
                    .to_string()
            }
            (Self::Aarch64, false) => {
                "ARM64 guest running under QEMU multi-threaded TCG software translation (no /dev/kvm)."
                    .to_string()
            }
            (Self::X86_64, _) => {
                "x86-64 guest on ARM64 host ALWAYS uses QEMU TCG binary emulation; native performance is NOT possible."
                    .to_string()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmState {
    Stopped,
    Starting,
    Running,
    Paused,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmAccelerator {
    TcgMulti,
    TcgSingle,
    Kvm,
}

impl VmAccelerator {
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::TcgMulti => "tcg",
            Self::TcgSingle => "tcg-single",
            Self::Kvm => "kvm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareKind {
    AavmfUefi,
    OvmfUefi,
    SeaBios,
    DirectKernel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareConfig {
    pub kind: FirmwareKind,
    pub code_path: Option<PathBuf>,
    pub vars_path: Option<PathBuf>,
    pub secure_boot: bool,
}

impl FirmwareConfig {
    pub fn default_for(arch: VmArch, vm_dir: &Path) -> Self {
        match arch {
            VmArch::Aarch64 => Self {
                kind: FirmwareKind::AavmfUefi,
                code_path: None,
                vars_path: Some(vm_dir.join("AAVMF_VARS.fd")),
                secure_boot: false,
            },
            VmArch::X86_64 => Self {
                kind: FirmwareKind::OvmfUefi,
                code_path: None,
                vars_path: Some(vm_dir.join("OVMF_VARS.fd")),
                secure_boot: false,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiskFormat {
    Qcow2,
    Raw,
}

impl DiskFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Qcow2 => "qcow2",
            Self::Raw => "raw",
        }
    }

    pub fn parse(s: &str) -> Result<Self, VmError> {
        match s.trim().to_ascii_lowercase().as_str() {
            "qcow2" => Ok(Self::Qcow2),
            "raw" | "img" => Ok(Self::Raw),
            other => Err(VmError::InvalidConfig(format!(
                "Unsupported disk format '{other}'; supported: qcow2, raw"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskBus {
    VirtioBlk,
    VirtioScsi,
    Sata,
    Ide,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDeviceConfig {
    pub disk_id: String,
    pub path: PathBuf,
    pub format: DiskFormat,
    pub bus: DiskBus,
    pub size_mb: u64,
    pub read_only: bool,
    pub boot_index: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    UserSlirp,
    IsolatedSlirp,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardRule {
    pub protocol: String,
    pub host_port: u16,
    pub guest_port: u16,
    pub bind_loopback_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub mode: NetworkMode,
    pub nic_model: String,
    pub guest_cidr: String,
    pub dns_server: String,
    pub port_forwards: Vec<PortForwardRule>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: NetworkMode::UserSlirp,
            nic_model: "virtio-net-pci".to_string(),
            guest_cidr: "10.0.2.0/24".to_string(),
            dns_server: "10.0.2.3".to_string(),
            port_forwards: vec![PortForwardRule {
                protocol: "tcp".to_string(),
                host_port: 2222,
                guest_port: 22,
                bind_loopback_only: true,
            }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmDisplayBackend {
    VncLocalhost,
    HeadlessSerial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayVmConfig {
    pub backend: VmDisplayBackend,
    pub gpu_device: String,
    pub vnc_display: u16,
    pub width: u32,
    pub height: u32,
    pub enable_usb_tablet: bool,
    pub enable_clipboard_note: String,
}

impl DisplayVmConfig {
    pub fn default_for(arch: VmArch, vnc_display: u16) -> Self {
        let gpu_device = match arch {
            VmArch::Aarch64 => "virtio-gpu-pci".to_string(),
            VmArch::X86_64 => "virtio-vga".to_string(),
        };
        Self {
            backend: VmDisplayBackend::VncLocalhost,
            gpu_device,
            vnc_display,
            width: 1280,
            height: 720,
            enable_usb_tablet: true,
            enable_clipboard_note: "VNC RFB 003.008 text cut-buffer supported when guest runs a VNC/X11 clipboard agent.".to_string(),
        }
    }

    pub fn vnc_port(&self) -> u16 {
        5900 + self.vnc_display
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmAudioBackend {
    Disabled,
    PulseLoopback,
    WavDebug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioVmConfig {
    pub backend: VmAudioBackend,
    pub device_model: String,
    pub pulse_server: String,
    pub limitation_note: String,
}

impl Default for AudioVmConfig {
    fn default() -> Self {
        Self {
            backend: VmAudioBackend::Disabled,
            device_model: "intel-hda".to_string(),
            pulse_server: "tcp:127.0.0.1:4713".to_string(),
            limitation_note: "QEMU audio over PulseAudio TCP loopback (127.0.0.1:4713) requires QEMU built with --audio-drv-list=pa and active Android AudioBridgePlayer; disabled by default until verified on device.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootConfig {
    pub boot_order: String,
    pub iso_path: Option<PathBuf>,
    pub direct_kernel: Option<PathBuf>,
    pub direct_initrd: Option<PathBuf>,
    pub kernel_cmdline: Option<String>,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            boot_order: "cdn".to_string(),
            iso_path: None,
            direct_kernel: None,
            direct_initrd: None,
            kernel_cmdline: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConfig {
    pub cores: u8,
    pub threads: u8,
    pub model: String,
    pub tcg_tb_size_mb: u32,
}

impl CpuConfig {
    pub fn default_for(arch: VmArch, cores: u8) -> Self {
        let model = match arch {
            VmArch::Aarch64 => "cortex-a72".to_string(),
            VmArch::X86_64 => "max".to_string(),
        };
        Self {
            cores: cores.clamp(1, 16),
            threads: 1,
            model,
            tcg_tb_size_mb: 128,
        }
    }
}

/// Complete Phase 5 persistent VM configuration profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmProfile {
    pub id: String,
    pub name: String,
    pub arch: VmArch,
    pub machine_type: String,
    pub cpu: CpuConfig,
    pub memory_mb: u32,
    pub accelerator_preference: VmAccelerator,
    pub firmware: FirmwareConfig,
    pub disks: Vec<StorageDeviceConfig>,
    pub network: NetworkConfig,
    pub display: DisplayVmConfig,
    pub audio: AudioVmConfig,
    pub boot: BootConfig,
    pub qmp_port: u16,
    pub qemu_version: String,
    pub created_at_epoch_ms: u64,
    pub last_used_at_epoch_ms: Option<u64>,
    pub state: VmState,
    pub notes: String,
}

impl VmProfile {
    pub fn default_preset(
        id: &str,
        name: &str,
        arch: VmArch,
        memory_mb: u32,
        cpus: u8,
        disk_size_mb: u64,
        disk_format: DiskFormat,
        vm_dir: &Path,
        vnc_display: u16,
        qmp_port: u16,
    ) -> Self {
        let now = now_epoch_ms();
        let disk_ext = if disk_format == DiskFormat::Qcow2 {
            "qcow2"
        } else {
            "img"
        };
        let machine_type = match arch {
            VmArch::Aarch64 => "virt,gic-version=3".to_string(),
            VmArch::X86_64 => "q35".to_string(),
        };
        let notes = match arch {
            VmArch::Aarch64 => {
                "ARM64 guest profile (QEMU virt + AAVMF UEFI + VirtIO). Uses multi-threaded TCG on non-KVM Android devices."
                    .to_string()
            }
            VmArch::X86_64 => {
                "x86-64 guest profile (QEMU q35 + OVMF/SeaBIOS + VirtIO). Software TCG emulation on ARM64 host — significantly slower than native ARM64."
                    .to_string()
            }
        };

        Self {
            id: id.to_string(),
            name: if name.trim().is_empty() {
                id.to_string()
            } else {
                name.trim().to_string()
            },
            arch,
            machine_type,
            cpu: CpuConfig::default_for(arch, cpus),
            memory_mb,
            accelerator_preference: VmAccelerator::TcgMulti,
            firmware: FirmwareConfig::default_for(arch, vm_dir),
            disks: vec![StorageDeviceConfig {
                disk_id: "disk0".to_string(),
                path: vm_dir.join(format!("disk0.{disk_ext}")),
                format: disk_format,
                bus: DiskBus::VirtioBlk,
                size_mb: disk_size_mb,
                read_only: false,
                boot_index: Some(1),
            }],
            network: NetworkConfig::default(),
            display: DisplayVmConfig::default_for(arch, vnc_display),
            audio: AudioVmConfig::default(),
            boot: BootConfig::default(),
            qmp_port,
            qemu_version: QEMU_PINNED_VERSION.to_string(),
            created_at_epoch_ms: now,
            last_used_at_epoch_ms: None,
            state: VmState::Stopped,
            notes,
        }
    }

    pub fn validate(&self) -> Result<(), VmError> {
        validate_vm_id(&self.id)?;
        if self.memory_mb < 256 || self.memory_mb > 16384 {
            return Err(VmError::InvalidConfig(format!(
                "memory_mb {} must be between 256 and 16384",
                self.memory_mb
            )));
        }
        if self.cpu.cores == 0 || self.cpu.cores > 16 {
            return Err(VmError::InvalidConfig(format!(
                "cpu.cores {} must be between 1 and 16",
                self.cpu.cores
            )));
        }
        if self.cpu.tcg_tb_size_mb < 16 || self.cpu.tcg_tb_size_mb > 1024 {
            return Err(VmError::InvalidConfig(format!(
                "tcg_tb_size_mb {} must be between 16 and 1024",
                self.cpu.tcg_tb_size_mb
            )));
        }
        if self.disks.is_empty() {
            return Err(VmError::InvalidConfig(
                "VM must have at least one primary storage disk".to_string(),
            ));
        }
        for disk in &self.disks {
            if disk.size_mb < 64 {
                return Err(VmError::InvalidConfig(format!(
                    "Disk '{}' size {} MB must be >= 64 MB",
                    disk.disk_id, disk.size_mb
                )));
            }
        }
        for rule in &self.network.port_forwards {
            if !matches!(rule.protocol.as_str(), "tcp" | "udp") {
                return Err(VmError::InvalidConfig(format!(
                    "Unsupported port forward protocol '{}'",
                    rule.protocol
                )));
            }
            if rule.host_port < 1024 {
                return Err(VmError::InvalidConfig(format!(
                    "Rootless Android forbids binding privileged host port {}",
                    rule.host_port
                )));
            }
        }
        Ok(())
    }

    pub fn primary_disk(&self) -> Option<&StorageDeviceConfig> {
        self.disks.first()
    }

    pub fn vnc_port(&self) -> u16 {
        self.display.vnc_port()
    }
}

/// Legacy Phase 1 `VmConfig` preserved for full backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub id: String,
    pub arch: VmArch,
    pub memory_mb: u32,
    pub cpus: u8,
    pub disk_size_mb: u32,
    pub disk_image_path: PathBuf,
    pub iso_path: Option<PathBuf>,
    pub vnc_display: u16,
    pub qmp_port: u16,
    pub enable_kvm: bool,
}

impl VmConfig {
    pub fn validate(&self) -> Result<(), VmError> {
        validate_vm_id(&self.id)?;
        if self.memory_mb < 256 || self.memory_mb > 16384 {
            return Err(VmError::InvalidConfig(format!(
                "memory_mb {} must be between 256 and 16384",
                self.memory_mb
            )));
        }
        if self.cpus == 0 || self.cpus > 16 {
            return Err(VmError::InvalidConfig(format!(
                "cpus {} must be between 1 and 16",
                self.cpus
            )));
        }
        if self.disk_size_mb < 256 {
            return Err(VmError::InvalidConfig(format!(
                "disk_size_mb {} must be >= 256 MB",
                self.disk_size_mb
            )));
        }
        Ok(())
    }

    pub fn vnc_port(&self) -> u16 {
        5900 + self.vnc_display
    }

    pub fn to_profile(&self) -> VmProfile {
        let vm_dir = self
            .disk_image_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let format = if self
            .disk_image_path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("qcow2"))
        {
            DiskFormat::Qcow2
        } else {
            DiskFormat::Raw
        };
        let mut profile = VmProfile::default_preset(
            &self.id,
            &self.id,
            self.arch,
            self.memory_mb,
            self.cpus,
            self.disk_size_mb as u64,
            format,
            &vm_dir,
            self.vnc_display,
            self.qmp_port,
        );
        profile.disks[0].path = self.disk_image_path.clone();
        profile.boot.iso_path = self.iso_path.clone();
        profile.accelerator_preference = if self.enable_kvm {
            VmAccelerator::Kvm
        } else {
            VmAccelerator::TcgMulti
        };
        profile
    }
}

pub fn validate_vm_id(id: &str) -> Result<(), VmError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(VmError::InvalidConfig(format!(
            "VM id '{}' must be 1..=64 ASCII alphanumeric/dash/underscore characters",
            id
        )));
    }
    Ok(())
}

pub fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
