use std::path::Path;
use crate::error::RuntimeError;
use crate::models::*;

/// Core runtime provider responsible for lifecycle and environment provisioning
pub trait RuntimeProvider: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn is_supported(&self, arch: Architecture) -> bool;
    fn health_check(&self) -> Result<HealthReport, RuntimeError>;
    fn check_ptrace_compatibility(&self) -> Result<bool, RuntimeError>;
}

/// Linux distribution metadata and behavior definition
pub trait LinuxDistribution: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn version(&self) -> &str;
    fn supported_architectures(&self) -> &[Architecture];
    fn default_desktop_environment(&self) -> &str;
    fn required_packages(&self) -> &[&str];
}

/// Individual rootless environment instance manager
pub trait EnvironmentManager: Send + Sync {
    fn environment_id(&self) -> &str;
    fn state(&self) -> EnvironmentState;
    fn initialize(&mut self) -> Result<(), RuntimeError>;
    fn start(&mut self) -> Result<(), RuntimeError>;
    fn stop(&mut self) -> Result<(), RuntimeError>;
    fn supervisor(&self) -> &dyn ProcessSupervisor;
}

/// Supervised process runner and lifecycle controller
pub trait ProcessSupervisor: Send + Sync {
    fn spawn_process(&self, command: &ProcessCommand) -> Result<ProcessId, RuntimeError>;
    fn send_signal(&self, pid: ProcessId, signal: Signal) -> Result<(), RuntimeError>;
    fn is_alive(&self, pid: ProcessId) -> bool;
    fn list_processes(&self) -> Vec<ProcessInfo>;
}

/// Terminal and PTY management interface
pub trait TerminalSession: Send + Sync {
    fn session_id(&self) -> &str;
    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), RuntimeError>;
    fn write_input(&mut self, data: &[u8]) -> Result<usize, RuntimeError>;
    fn read_output(&mut self, buf: &mut [u8]) -> Result<usize, RuntimeError>;
    fn close(&mut self) -> Result<(), RuntimeError>;
}

/// Display server backend interface (Xvnc, Termux:X11, Framebuffer)
pub trait DisplayBackend: Send + Sync {
    fn backend_type(&self) -> DisplayType;
    fn is_running(&self) -> bool;
    fn start(&mut self, config: &DisplayConfig) -> Result<DisplayEndpoint, RuntimeError>;
    fn stop(&mut self) -> Result<(), RuntimeError>;
    fn endpoint(&self) -> Option<DisplayEndpoint>;
}

/// Artifact discovery, download URL resolution, and cryptographic verification
pub trait ArtifactProvider: Send + Sync {
    fn get_artifact_url(&self, artifact_name: &str, arch: Architecture) -> Result<String, RuntimeError>;
    fn verify_artifact(&self, path: &Path, expected_sha256: &str) -> Result<bool, RuntimeError>;
}
