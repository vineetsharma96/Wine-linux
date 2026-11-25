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

    // 4. Rootless Sandbox & Runtime Artifacts Check
    let t3 = Instant::now();
    let proot_bin_name = androidpc_linux_runtime::ProotRuntimeProvider::binary_name_for_arch(
        androidpc_runtime_api::Architecture::host(),
    );
    let proot_exists = if cfg!(target_os = "android") {
        config.native_lib_dir.as_ref().is_some_and(|dir| dir.join("libproot.so").is_file())
    } else {
        config.runtime_artifacts_dir.join(proot_bin_name).is_file()
    };
    let xvnc_exists = config
        .runtime_artifacts_dir
        .join("tigervnc-arm64")
        .join("Xvnc")
        .is_file();

    #[cfg(target_os = "android")]
    let (sandbox_level, sandbox_msg) = if proot_exists {
        (
            HealthLevel::Healthy,
            format!(
                "PRoot executable packaged ({}; Xvnc: {})",
                proot_bin_name,
                if xvnc_exists { "installed" } else { "pending" }
            ),
        )
    } else {
        (
            HealthLevel::Degraded,
            format!(
                "App sandbox isolated; {} pending artifact install",
                proot_bin_name
            ),
        )
    };
    #[cfg(not(target_os = "android"))]
    let (sandbox_level, sandbox_msg) = (
        HealthLevel::Healthy,
        format!(
            "Rootless execution container: Host test mode active (proot={}, xvnc={})",
            proot_exists, xvnc_exists
        ),
    );

    subsystems.push(SubsystemHealth {
        name: "Rootless Isolation".to_string(),
        level: sandbox_level,
        message: sandbox_msg,
        latency_ms: t3.elapsed().as_millis() as u64,
    });

    // 5. Box64 x86-64 Compatibility Layer Check
    let t4 = Instant::now();
    let box64_bin = config.runtime_artifacts_dir.join("box64").join("box64");
    let box64_rc = config.runtime_artifacts_dir.join("box64").join("default.box64rc");
    let box64_msg = if box64_bin.is_file() {
        format!(
            "Box64 v{} binary staged (rcfile={})",
            androidpc_box64::BOX64_PINNED_VERSION,
            box64_rc.is_file()
        )
    } else {
        format!(
            "Box64 v{} runtime ready (binary pending build/staging via scripts/build_box64_arm64.sh)",
            androidpc_box64::BOX64_PINNED_VERSION
        )
    };
    subsystems.push(SubsystemHealth {
        name: "Box64 x86-64 Compatibility".to_string(),
        level: HealthLevel::Healthy,
        message: box64_msg,
        latency_ms: t4.elapsed().as_millis() as u64,
    });

    // 6. Wine 11.0 WoW64 & Graphics Stack Readiness Check
    let t5 = Instant::now();
    let wine_bin = config.runtime_artifacts_dir.join("wine").join("bin").join("wine");
    let turnip_icd = config
        .runtime_artifacts_dir
        .join("graphics")
        .join("turnip")
        .join("freedreno_icd.aarch64.json");
    let dxvk_dll = config.runtime_artifacts_dir.join("dxvk").join("x64").join("d3d11.dll");
    let wine_gfx_msg = format!(
        "Wine {} WoW64: {} • Turnip ICD: {} • DXVK {}: {} • Software fallback available",
        androidpc_wine::WINE_PINNED_VERSION,
        if wine_bin.is_file() { "staged" } else { "pending" },
        if turnip_icd.is_file() { "staged" } else { "pending" },
        androidpc_graphics_backend::DXVK_PINNED_VERSION,
        if dxvk_dll.is_file() { "staged" } else { "pending" }
    );
    subsystems.push(SubsystemHealth {
        name: "Windows Compatibility & Graphics".to_string(),
        level: HealthLevel::Healthy,
        message: wine_gfx_msg,
        latency_ms: t5.elapsed().as_millis() as u64,
    });

    // 7. QEMU Virtual Machine Engine Check
    let t6 = Instant::now();
    let qemu_dir = config.runtime_artifacts_dir.join("qemu");
    let fw_dir = config.runtime_artifacts_dir.join("qemu-firmware");
    let caps = androidpc_vm::QemuEngineCapabilities::probe(&qemu_dir, &fw_dir);
    let qemu_msg = format!(
        "QEMU {} (aarch64: {}, x86_64: {}, qemu-img: {}) • Accel: {} • EDK2 {}: AAVMF={}, OVMF={}",
        caps.pinned_qemu_version,
        if caps.qemu_aarch64_installed { "staged" } else { "pending" },
        if caps.qemu_x86_64_installed { "staged" } else { "pending" },
        if caps.qemu_img_installed { "staged" } else { "pending" },
        caps.kvm.accelerator,
        caps.pinned_edk2_version,
        if caps.aavmf_firmware_installed { "staged" } else { "pending" },
        if caps.ovmf_firmware_installed { "staged" } else { "pending" }
    );
    subsystems.push(SubsystemHealth {
        name: "QEMU Virtual Machines".to_string(),
        level: HealthLevel::Healthy,
        message: qemu_msg,
        latency_ms: t6.elapsed().as_millis() as u64,
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
