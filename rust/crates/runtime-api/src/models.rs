use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Architecture {
    #[serde(rename = "aarch64")]
    Aarch64,
    #[serde(rename = "x86_64")]
    X86_64,
    #[serde(rename = "armv7")]
    Armv7,
    #[serde(rename = "i686")]
    I686,
    #[serde(rename = "riscv64")]
    Riscv64,
}

impl Architecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::X86_64 => "x86_64",
            Self::Armv7 => "armv7",
            Self::I686 => "i686",
            Self::Riscv64 => "riscv64",
        }
    }

    pub fn host() -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            Self::Aarch64
        }
        #[cfg(target_arch = "x86_64")]
        {
            Self::X86_64
        }
        #[cfg(target_arch = "arm")]
        {
            Self::Armv7
        }
        #[cfg(target_arch = "x86")]
        {
            Self::I686
        }
        #[cfg(target_arch = "riscv64")]
        {
            Self::Riscv64
        }
        #[cfg(not(any(
            target_arch = "aarch64",
            target_arch = "x86_64",
            target_arch = "arm",
            target_arch = "x86",
            target_arch = "riscv64"
        )))]
        {
            Self::Aarch64
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvironmentState {
    Uninitialized,
    Provisioning,
    Stopped,
    Starting,
    Running,
    Stopping,
    Degraded,
    Error,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealth {
    pub name: String,
    pub level: HealthLevel,
    pub message: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub status: HealthLevel,
    pub subsystems: Vec<SubsystemHealth>,
    pub timestamp_epoch_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatusInfo {
    pub state: String,
    pub version: String,
    pub build_profile: String,
    pub host_arch: Architecture,
    pub active_environments: usize,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDiagnostics {
    pub engine_version: String,
    pub host_arch: String,
    pub target_os: String,
    pub rustc_version: String,
    pub memory_allocated_bytes: u64,
    pub available_storage_bytes: u64,
    pub active_processes_count: usize,
    pub rootless_ptrace_supported: bool,
    pub seccomp_restricted: bool,
    pub proot_ready: bool,
    pub reports: Vec<SubsystemHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<String>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

pub type ProcessId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Hup,
    Int,
    Quit,
    Kill,
    Term,
    Usr1,
    Usr2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: ProcessId,
    pub command: String,
    pub running: bool,
    pub memory_rss_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayType {
    Xvnc,
    TermuxX11,
    CustomFramebuffer,
    WaylandBridge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub display_type: DisplayType,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub display_number: u32,
    pub listen_address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayEndpoint {
    pub display_type: DisplayType,
    pub address: String,
    pub port: u16,
    pub auth_required: bool,
}
