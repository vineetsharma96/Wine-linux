use androidpc_configuration::EngineConfig;
use androidpc_linux_runtime::ProotRuntimeProvider;
use androidpc_runtime_api::{Architecture, HealthReport, SystemDiagnostics};
use crate::ENGINE_VERSION;

pub fn collect_system_diagnostics(
    config: &EngineConfig,
    health: &HealthReport,
    active_processes_count: usize,
) -> SystemDiagnostics {
    let proot_version = probe_proot_version(config);

    let available_storage_bytes = query_available_storage_bytes(&config.base_data_dir);

    SystemDiagnostics {
        engine_version: ENGINE_VERSION.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        target_os: std::env::consts::OS.to_string(),
        rustc_version: env!("CARGO_PKG_VERSION").to_string(),
        memory_allocated_bytes: process_memory_bytes(),
        available_storage_bytes,
        active_processes_count,
        rootless_ptrace_supported: false,
        seccomp_restricted: proc_status_field("Seccomp:").and_then(|value| value.parse::<u32>().ok()).is_some_and(|mode| mode != 0),
        proot_ready: proot_version.is_some(),
        proot_version,
        android_release: android_release(),
        last_guest_check_epoch_ms: None,
        reports: health.subsystems.clone(),
    }
}

pub(crate) fn probe_proot_version(config: &EngineConfig) -> Option<String> {
    let proot_path = if cfg!(target_os = "android") {
        config.native_lib_dir.as_ref()?.join("libproot.so")
    } else {
        config.runtime_artifacts_dir.join(ProotRuntimeProvider::binary_name_for_arch(Architecture::host()))
    };
    if !proot_path.is_file() {
        return None;
    }
    let mut paths = vec![config.runtime_artifacts_dir.join("lib")];
    paths.extend(proot_path.parent().map(std::path::Path::to_path_buf));
    let output = std::process::Command::new(proot_path)
        .env_clear()
        .env("LD_LIBRARY_PATH", std::env::join_paths(paths).ok()?)
        .arg("--version")
        .output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).split_whitespace()
        .find(|part| part.contains('.') && part.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
        .map(str::to_owned)
}

fn proc_status_field(key: &str) -> Option<String> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| line.strip_prefix(key))
        .and_then(|value| value.split_whitespace().next()).map(str::to_owned)
}

fn process_memory_bytes() -> u64 {
    proc_status_field("VmRSS:").and_then(|value| value.parse::<u64>().ok())
        .map_or(0, |kib| kib.saturating_mul(1024))
}

#[cfg(target_os = "android")]
fn android_release() -> Option<String> {
    let mut value = [0; 92];
    let len = unsafe { libc::__system_property_get(c"ro.build.version.release".as_ptr(), value.as_mut_ptr()) };
    (len > 0).then(|| unsafe { std::ffi::CStr::from_ptr(value.as_ptr()) }.to_string_lossy().into_owned())
}

#[cfg(not(target_os = "android"))]
fn android_release() -> Option<String> {
    None
}

#[cfg(unix)]
pub(crate) fn query_available_storage_bytes(path: &std::path::Path) -> u64 {
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
    0
}
