use androidpc_configuration::EngineConfig;
use androidpc_linux_runtime::ProotRuntimeProvider;
use androidpc_runtime_api::{Architecture, HealthReport, SystemDiagnostics};
use crate::ENGINE_VERSION;

pub fn collect_system_diagnostics(
    config: &EngineConfig,
    health: &HealthReport,
    active_processes_count: usize,
) -> SystemDiagnostics {
    let proot_name = ProotRuntimeProvider::binary_name_for_arch(Architecture::host());
    let proot_path = config.runtime_artifacts_dir.join(proot_name);
    let proot_ready = proot_path.is_file() || cfg!(not(target_os = "android"));

    let available_storage_bytes = query_available_storage_bytes(&config.base_data_dir);

    SystemDiagnostics {
        engine_version: ENGINE_VERSION.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        target_os: std::env::consts::OS.to_string(),
        rustc_version: env!("CARGO_PKG_VERSION").to_string(),
        memory_allocated_bytes: 0,
        available_storage_bytes,
        active_processes_count,
        rootless_ptrace_supported: true,
        seccomp_restricted: cfg!(target_os = "android"),
        proot_ready,
        reports: health.subsystems.clone(),
    }
}

#[cfg(unix)]
fn query_available_storage_bytes(path: &std::path::Path) -> u64 {
    use std::ffi::CString;
    let probe = if path.exists() {
        path
    } else {
        std::path::Path::new("/data/local/tmp")
    };
    if let Ok(c_path) = CString::new(probe.to_string_lossy().as_bytes()) {
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                return (stat.f_bavail as u64) * (stat.f_frsize as u64);
            }
        }
    }
    0
}

#[cfg(not(unix))]
fn query_available_storage_bytes(_path: &std::path::Path) -> u64 {
    // Non-unix host fallback
    1024 * 1024 * 1024 * 10
}
