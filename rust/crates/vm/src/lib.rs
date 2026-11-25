use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

use androidpc_process::DefaultProcessSupervisor;
use androidpc_runtime_api::{ProcessCommand, ProcessId, ProcessSupervisor, RuntimeError, Signal};

#[derive(Debug, Error)]
pub enum VmError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),

    #[error("Invalid VM configuration: {0}")]
    InvalidConfig(String),

    #[error("VM not found: {0}")]
    NotFound(String),

    #[error("VM already exists: {0}")]
    AlreadyExists(String),

    #[error("VM '{0}' is already running")]
    AlreadyRunning(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmState {
    Stopped,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvmCapabilities {
    pub device_present: bool,
    pub accessible: bool,
    pub accelerator: String,
    pub notes: String,
}

impl KvmCapabilities {
    /// Probes `/dev/kvm` on the host without requesting root privileges.
    pub fn probe() -> Self {
        let kvm_path = Path::new("/dev/kvm");
        let present = kvm_path.exists();
        if !present {
            return Self {
                device_present: false,
                accessible: false,
                accelerator: "tcg".to_string(),
                notes: "/dev/kvm not exposed by kernel; using TCG software emulation".to_string(),
            };
        }

        match OpenOptions::new().read(true).write(true).open(kvm_path) {
            Ok(_) => Self {
                device_present: true,
                accessible: true,
                accelerator: "kvm".to_string(),
                notes: "/dev/kvm accessible; hardware-assisted virtualization available".to_string(),
            },
            Err(err) => Self {
                device_present: true,
                accessible: false,
                accelerator: "tcg".to_string(),
                notes: format!("/dev/kvm present but restricted ({err}); falling back to TCG"),
            },
        }
    }
}

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
        if self.id.is_empty()
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(VmError::InvalidConfig(format!(
                "VM id '{}' must be non-empty alphanumeric/dash/underscore",
                self.id
            )));
        }
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
}

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

pub struct QmpClient;

impl QmpClient {
    /// Sends QMP capability negotiation and `query-status` to a running QEMU instance.
    pub fn query_status(qmp_port: u16) -> Result<String, VmError> {
        use std::io::{Read, Write};
        use std::net::{SocketAddr, TcpStream};
        use std::time::Duration;

        let addr: SocketAddr = ([127, 0, 0, 1], qmp_port).into();
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));

        let mut buf = [0u8; 2048];
        let _ = stream.read(&mut buf);
        stream.write_all(b"{\"execute\":\"qmp_capabilities\"}\n")?;
        let _ = stream.read(&mut buf);
        stream.write_all(b"{\"execute\":\"query-status\"}\n")?;
        let n = stream.read(&mut buf).unwrap_or(0);
        Ok(String::from_utf8_lossy(&buf[..n]).to_string())
    }
}

pub struct QemuCommandBuilder;

impl QemuCommandBuilder {
    /// Builds deterministic, hardened QEMU CLI arguments for Android userspace.
    pub fn build_args(config: &VmConfig, kvm_accessible: bool) -> Vec<String> {
        let use_kvm = config.enable_kvm && kvm_accessible;
        let mut args = Vec::new();

        match config.arch {
            VmArch::Aarch64 => {
                args.extend(["-machine".to_string(), "virt".to_string()]);
                let cpu = if use_kvm { "host" } else { "cortex-a57" };
                args.extend(["-cpu".to_string(), cpu.to_string()]);
            }
            VmArch::X86_64 => {
                args.extend(["-machine".to_string(), "q35".to_string()]);
                let cpu = if use_kvm { "host" } else { "max" };
                args.extend(["-cpu".to_string(), cpu.to_string()]);
            }
        }

        if use_kvm {
            args.extend(["-accel".to_string(), "kvm".to_string()]);
        } else {
            args.extend(["-accel".to_string(), "tcg,thread=multi".to_string()]);
        }

        args.extend(["-m".to_string(), config.memory_mb.to_string()]);
        args.extend(["-smp".to_string(), config.cpus.to_string()]);

        // VirtIO block disk image
        args.extend([
            "-drive".to_string(),
            format!(
                "file={},if=virtio,format=raw",
                config.disk_image_path.display()
            ),
        ]);

        // Optional boot ISO
        if let Some(ref iso) = config.iso_path {
            args.extend(["-cdrom".to_string(), iso.display().to_string()]);
            args.extend(["-boot".to_string(), "d".to_string()]);
        }

        // VirtIO user-mode networking (rootless SLIRP) with SSH port forward
        args.extend([
            "-netdev".to_string(),
            "user,id=net0,hostfwd=tcp:127.0.0.1:2222-:22".to_string(),
            "-device".to_string(),
            "virtio-net-pci,netdev=net0".to_string(),
        ]);

        // Localhost VNC display (integrates directly with AndroidPC RFB viewer)
        args.extend([
            "-display".to_string(),
            format!("vnc=127.0.0.1:{}", config.vnc_display),
        ]);

        // QMP management socket on localhost
        args.extend([
            "-qmp".to_string(),
            format!("tcp:127.0.0.1:{},server,nowait", config.qmp_port),
        ]);

        args.push("-no-reboot".to_string());
        args
    }
}

struct ManagedVm {
    config: VmConfig,
    state: VmState,
    pid: Option<ProcessId>,
}

pub struct QemuSupervisor {
    vms_dir: PathBuf,
    qemu_bin_dir: PathBuf,
    supervisor: DefaultProcessSupervisor,
    vms: RwLock<HashMap<String, ManagedVm>>,
}

impl QemuSupervisor {
    pub fn new(base_data_dir: &Path, qemu_bin_dir: &Path) -> Self {
        let vms_dir = base_data_dir.join("vms");
        let _ = std::fs::create_dir_all(&vms_dir);
        let mgr = Self {
            vms_dir,
            qemu_bin_dir: qemu_bin_dir.to_path_buf(),
            supervisor: DefaultProcessSupervisor::new(),
            vms: RwLock::new(HashMap::new()),
        };
        mgr.reload_persisted();
        mgr
    }

    fn reload_persisted(&self) {
        let Ok(entries) = std::fs::read_dir(&self.vms_dir) else {
            return;
        };
        let Ok(mut guard) = self.vms.write() else {
            return;
        };
        for entry in entries.flatten() {
            let cfg_path = entry.path().join("vm.json");
            if let Ok(raw) = std::fs::read_to_string(&cfg_path) {
                if let Ok(cfg) = serde_json::from_str::<VmConfig>(&raw) {
                    guard.insert(
                        cfg.id.clone(),
                        ManagedVm {
                            config: cfg,
                            state: VmState::Stopped,
                            pid: None,
                        },
                    );
                }
            }
        }
    }

    pub fn create_vm(
        &self,
        id: &str,
        arch: VmArch,
        memory_mb: u32,
        cpus: u8,
        disk_size_mb: u32,
    ) -> Result<VmStatusInfo, VmError> {
        let vm_dir = self.vms_dir.join(id);
        let disk_path = vm_dir.join("disk.img");

        let next_idx = self.vms.read().map(|g| g.len() as u16).unwrap_or(0);
        let config = VmConfig {
            id: id.to_string(),
            arch,
            memory_mb,
            cpus,
            disk_size_mb,
            disk_image_path: disk_path.clone(),
            iso_path: None,
            vnc_display: 10 + next_idx, // ports 5910+ to avoid collision with Xvnc :1 (5901)
            qmp_port: 4444 + next_idx,
            enable_kvm: true,
        };
        config.validate()?;

        {
            let guard = self
                .vms
                .read()
                .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
            if guard.contains_key(id) {
                return Err(VmError::AlreadyExists(id.to_string()));
            }
        }

        std::fs::create_dir_all(&vm_dir)?;
        // Create sparse raw disk image so it consumes negligible storage until written
        let disk_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&disk_path)?;
        disk_file.set_len((disk_size_mb as u64) * 1024 * 1024)?;

        let cfg_json = serde_json::to_string_pretty(&config)?;
        std::fs::write(vm_dir.join("vm.json"), cfg_json)?;

        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        guard.insert(
            id.to_string(),
            ManagedVm {
                config: config.clone(),
                state: VmState::Stopped,
                pid: None,
            },
        );

        info!("Created QEMU VM '{id}' ({:?}, {} MB RAM, {} vCPU)", arch, memory_mb, cpus);
        Ok(Self::to_status(&config, VmState::Stopped, None))
    }

    pub fn start_vm(&self, id: &str) -> Result<VmStatusInfo, VmError> {
        let kvm = KvmCapabilities::probe();
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(id)
            .ok_or_else(|| VmError::NotFound(id.to_string()))?;

        if let Some(pid) = vm.pid {
            if self.supervisor.is_alive(pid) {
                return Err(VmError::AlreadyRunning(id.to_string()));
            }
        }

        let bin_name = vm.config.arch.qemu_binary();
        let qemu_bin = self.qemu_bin_dir.join(bin_name);
        let program = if qemu_bin.exists() {
            qemu_bin.to_string_lossy().to_string()
        } else {
            bin_name.to_string()
        };

        let args = QemuCommandBuilder::build_args(&vm.config, kvm.accessible);
        let cmd = ProcessCommand {
            program,
            args,
            env: Vec::new(),
            cwd: Some(self.vms_dir.join(id).to_string_lossy().to_string()),
            uid: None,
            gid: None,
        };

        match self.supervisor.spawn_process(&cmd) {
            Ok(pid) => {
                vm.pid = Some(pid);
                vm.state = VmState::Running;
                info!("Started QEMU VM '{id}' with PID {pid}");
                Ok(Self::to_status(&vm.config, vm.state, vm.pid))
            }
            Err(err) => {
                warn!("Failed to spawn QEMU binary for '{id}': {err}");
                vm.state = VmState::Error;
                Err(VmError::Runtime(err))
            }
        }
    }

    pub fn stop_vm(&self, id: &str) -> Result<(), VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        let vm = guard
            .get_mut(id)
            .ok_or_else(|| VmError::NotFound(id.to_string()))?;

        if let Some(pid) = vm.pid.take() {
            let _ = self.supervisor.send_signal(pid, Signal::Term);
        }
        vm.state = VmState::Stopped;
        info!("Stopped QEMU VM '{id}'");
        Ok(())
    }

    pub fn delete_vm(&self, id: &str) -> Result<(), VmError> {
        let mut guard = self
            .vms
            .write()
            .map_err(|_| VmError::InvalidConfig("Lock poisoned".into()))?;
        if let Some(mut vm) = guard.remove(id) {
            if let Some(pid) = vm.pid.take() {
                let _ = self.supervisor.send_signal(pid, Signal::Kill);
            }
        }
        let vm_dir = self.vms_dir.join(id);
        if vm_dir.exists() {
            std::fs::remove_dir_all(vm_dir)?;
        }
        info!("Deleted QEMU VM '{id}'");
        Ok(())
    }

    pub fn list_vms(&self) -> Vec<VmStatusInfo> {
        let Ok(mut guard) = self.vms.write() else {
            return Vec::new();
        };
        guard
            .values_mut()
            .map(|vm| {
                if vm.state == VmState::Running {
                    if let Some(pid) = vm.pid {
                        if !self.supervisor.is_alive(pid) {
                            vm.state = VmState::Stopped;
                            vm.pid = None;
                        }
                    }
                }
                Self::to_status(&vm.config, vm.state, vm.pid)
            })
            .collect()
    }

    fn to_status(config: &VmConfig, state: VmState, pid: Option<ProcessId>) -> VmStatusInfo {
        let kvm = KvmCapabilities::probe();
        let accel = if config.enable_kvm && kvm.accessible {
            "kvm".to_string()
        } else {
            "tcg".to_string()
        };
        VmStatusInfo {
            id: config.id.clone(),
            arch: config.arch,
            state,
            memory_mb: config.memory_mb,
            cpus: config.cpus,
            disk_size_mb: config.disk_size_mb,
            vnc_port: config.vnc_port(),
            qmp_port: config.qmp_port,
            accelerator: accel,
            pid,
            disk_exists: config.disk_image_path.exists(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(tcg_args.contains(&"virt".to_string()));
        assert!(tcg_args.contains(&"cortex-a57".to_string()));
        assert!(tcg_args.contains(&"tcg,thread=multi".to_string()));
        assert!(tcg_args.contains(&"vnc=127.0.0.1:10".to_string()));

        let kvm_args = QemuCommandBuilder::build_args(&cfg, true);
        assert!(kvm_args.contains(&"host".to_string()));
        assert!(kvm_args.contains(&"kvm".to_string()));
    }

    #[test]
    fn test_qemu_command_builder_x86_64() {
        let cfg = VmConfig {
            id: "win10-x64".to_string(),
            arch: VmArch::X86_64,
            memory_mb: 2048,
            cpus: 2,
            disk_size_mb: 4096,
            disk_image_path: PathBuf::from("/data/vms/win10/disk.img"),
            iso_path: Some(PathBuf::from("/data/isos/boot.iso")),
            vnc_display: 11,
            qmp_port: 4445,
            enable_kvm: false,
        };
        let args = QemuCommandBuilder::build_args(&cfg, true);
        assert!(args.contains(&"q35".to_string()));
        assert!(args.contains(&"max".to_string()));
        assert!(args.contains(&"-cdrom".to_string()));
        assert_eq!(cfg.vnc_port(), 5911);
    }

    #[test]
    fn test_vm_supervisor_create_list_delete() {
        let tmp = std::env::temp_dir().join("androidpc_vm_test_1");
        let _ = std::fs::remove_dir_all(&tmp);
        let sup = QemuSupervisor::new(&tmp, Path::new("/nonexistent/bin"));

        let created = sup
            .create_vm("test-vm", VmArch::Aarch64, 512, 2, 512)
            .expect("create vm");
        assert_eq!(created.id, "test-vm");
        assert_eq!(created.vnc_port, 5910);
        assert!(created.disk_exists);

        let list = sup.list_vms();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].state, VmState::Stopped);

        sup.delete_vm("test-vm").expect("delete vm");
        assert!(sup.list_vms().is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
