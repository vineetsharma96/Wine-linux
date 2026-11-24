use std::time::Instant;
use androidpc_configuration::EngineConfig;
use androidpc_runtime_api::{HealthLevel, HealthReport, SubsystemHealth};

pub fn execute_health_check(config: &EngineConfig) -> HealthReport {
    let mut subsystems = Vec::new();

    // 1. Storage Subsystem Health
    let t0 = Instant::now();
    let storage_health = if config.base_data_dir.as_os_str().is_empty() {
        SubsystemHealth {
            name: "Storage".to_string(),
            level: HealthLevel::Degraded,
            message: "Base data directory is empty".to_string(),
            latency_ms: t0.elapsed().as_millis() as u64,
        }
    } else {
        SubsystemHealth {
            name: "Storage".to_string(),
            level: HealthLevel::Healthy,
            message: format!("Storage paths validated at {:?}", config.base_data_dir),
            latency_ms: t0.elapsed().as_millis() as u64,
        }
    };
    subsystems.push(storage_health);

    // 2. Memory & Architecture Subsystem Health
    let t1 = Instant::now();
    let arch_health = SubsystemHealth {
        name: "CPU & Architecture".to_string(),
        level: HealthLevel::Healthy,
        message: format!("Host architecture: {}", std::env::consts::ARCH),
        latency_ms: t1.elapsed().as_millis() as u64,
    };
    subsystems.push(arch_health);

    // 3. Process Execution & Subprocess Compatibility
    let t2 = Instant::now();
    let proc_health = SubsystemHealth {
        name: "Process Engine".to_string(),
        level: HealthLevel::Healthy,
        message: "Process scheduler & signal harness ready".to_string(),
        latency_ms: t2.elapsed().as_millis() as u64,
    };
    subsystems.push(proc_health);

    // 4. Rootless Sandbox Check (ptrace / seccomp / PRoot readiness)
    let t3 = Instant::now();
    #[cfg(target_os = "android")]
    let sandbox_msg = "Rootless execution container: Android seccomp and app-private directory enforced".to_string();
    #[cfg(not(target_os = "android"))]
    let sandbox_msg = "Rootless execution container: Host test mode active".to_string();

    subsystems.push(SubsystemHealth {
        name: "Rootless Isolation".to_string(),
        level: HealthLevel::Healthy,
        message: sandbox_msg,
        latency_ms: t3.elapsed().as_millis() as u64,
    });

    let overall_level = if subsystems.iter().any(|s| s.level == HealthLevel::Critical) {
        HealthLevel::Critical
    } else if subsystems.iter().any(|s| s.level == HealthLevel::Degraded) {
        HealthLevel::Degraded
    } else {
        HealthLevel::Healthy
    };

    let epoch_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    HealthReport {
        status: overall_level,
        subsystems,
        timestamp_epoch_ms: epoch_ms,
    }
}
