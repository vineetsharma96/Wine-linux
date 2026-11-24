use androidpc_configuration::EngineConfig;
use androidpc_runtime_api::{HealthReport, SystemDiagnostics};
use crate::ENGINE_VERSION;

pub fn collect_system_diagnostics(
    _config: &EngineConfig,
    health: &HealthReport,
) -> SystemDiagnostics {
    SystemDiagnostics {
        engine_version: ENGINE_VERSION.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        target_os: std::env::consts::OS.to_string(),
        rustc_version: env!("CARGO_PKG_VERSION").to_string(),
        memory_allocated_bytes: 0,
        available_storage_bytes: 1024 * 1024 * 1024 * 10, // Placeholder for free disk space check
        active_processes_count: 0,
        rootless_ptrace_supported: true,
        seccomp_restricted: true,
        proot_ready: true,
        reports: health.subsystems.clone(),
    }
}
