package com.androidpc.environment.bridge

data class EngineStatusData(
    val state: String = "Unknown",
    val version: String = "Unknown",
    val build_profile: String = "Unknown",
    val host_arch: String = "Unknown",
    val active_environments: Int = 0,
    val uptime_seconds: Long = 0L
)

data class SubsystemHealthData(
    val name: String,
    val level: String,
    val message: String,
    val latency_ms: Long
)

data class HealthReportData(
    val status: String,
    val subsystems: List<SubsystemHealthData> = emptyList(),
    val timestamp_epoch_ms: Long = 0L
)

data class SystemDiagnosticsData(
    val engine_version: String,
    val host_arch: String,
    val target_os: String,
    val rustc_version: String,
    val memory_allocated_bytes: Long,
    val available_storage_bytes: Long,
    val active_processes_count: Int,
    val rootless_ptrace_supported: Boolean,
    val seccomp_restricted: Boolean,
    val proot_ready: Boolean,
    val proot_version: String? = null,
    val android_release: String? = null,
    val last_guest_check_epoch_ms: Long? = null,
    val reports: List<SubsystemHealthData> = emptyList()
)

data class LogEntryData(
    val timestamp_epoch_ms: Long,
    val level: String,
    val target: String,
    val message: String
)

data class EngineConfigData(
    val base_data_dir: String,
    val runtime_artifacts_dir: String,
    val temporary_dir: String,
    val native_lib_dir: String? = null,
    val trusted_rootfs_key_hex: String? = null,
    val app_version_code: Int,
    val debug_mode: Boolean,
    val log_level: String,
    val max_concurrent_processes: Int
)

data class DisplayEndpointData(
    val display_type: String = "Xvnc",
    val address: String = "127.0.0.1",
    val port: Int = 5901,
    val auth_required: Boolean = false,
    val session_id: String? = null
)

data class ArtifactMetadataData(
    val name: String = "",
    val distribution: String = "",
    val architecture: String = "",
    val filename: String = "",
    val url: String = "",
    val sha256: String = "",
    val size_bytes: Long = 0L
)

data class EnvironmentConfigData(
    val id: String = "",
    val display_name: String? = null,
    val last_started_at_epoch_ms: Long? = null,
    val runtime_version: String? = null,
    val distribution: String = "",
    val version: String = "",
    val architecture: String = "",
    val rootfs_directory: String = "",
    val default_username: String = "user",
    val memory_limit_mb: Long? = null
)

data class EnvironmentMetricsData(
    val processes: Int,
    val terminal_sessions: Int,
    val memory_rss_bytes: Long? = null,
    val cpu_time_ms: Long? = null,
    val seconds_since_launch: Long? = null
)

data class BackupData(
    val id: String,
    val created_at_epoch_ms: Long,
    val size_bytes: Long,
    val source_name: String,
    val distribution: String
)

data class BackupOperationData(
    val backups: List<BackupData>? = null,
    val environment_id: String? = null,
    val error: String? = null
)

data class PackageOperationData(
    val exit_code: Int? = null,
    val cancelled: Boolean = false,
    val stdout: String? = null,
    val stderr: String? = null,
    val error: String? = null
)

data class WineConfigData(
    val prefix_path: String = "/home/user/.wine",
    val arch: String = "win64",
    val windows_version: String = "win10",
    val dxvk_enabled: Boolean = true,
    val esync_enabled: Boolean = false,
    val debug_channels: String = "fixme-all,err-all",
    val display: String = ":1",
    val pulse_server: String = "127.0.0.1:4713"
)

data class WineStatusData(
    val prefix_path: String = "",
    val arch: String = "win64",
    val state: String = "Uninitialized",
    val prefix_exists: Boolean = false,
    val wineserver_running: Boolean = false,
    val active_wine_pids: List<Long> = emptyList(),
    val box64_dynarec: Boolean = true
)

data class ProcessCommandData(
    val program: String = "",
    val args: List<String> = emptyList(),
    val cwd: String? = null,
    val spawned_pid: Long? = null,
    val error: String? = null
)

data class AudioStatusData(
    val running: Boolean = false,
    val listen_address: String = "127.0.0.1",
    val port: Int = 4713,
    val sample_rate_hz: Int = 44100,
    val channels: Int = 2,
    val active_clients: Long = 0L,
    val buffered_bytes: Int = 0,
    val total_bytes_received: Long = 0L
)

data class KvmCapabilitiesData(
    val device_present: Boolean = false,
    val accessible: Boolean = false,
    val accelerator: String = "tcg",
    val notes: String = ""
)

data class VmStatusData(
    val id: String = "",
    val arch: String = "aarch64",
    val state: String = "Stopped",
    val memory_mb: Int = 1024,
    val cpus: Int = 2,
    val disk_size_mb: Int = 2048,
    val vnc_port: Int = 5910,
    val qmp_port: Int = 4444,
    val accelerator: String = "tcg",
    val pid: Long? = null,
    val disk_exists: Boolean = false
)

data class Box64InstallationStatusData(
    val installed: Boolean = false,
    val binary_path: String? = null,
    val detected_version: String? = null,
    val pinned_version: String = "0.4.4",
    val license: String = "MIT",
    val upstream_repo: String = "https://github.com/ptitSeb/box64",
    val host_arch: String = "aarch64",
    val dynarec_supported_on_host: Boolean = true,
    val abi_isolation_verified: Boolean = true,
    val status_summary: String = ""
)

data class Box64AppOverrideData(
    val dynarec_enabled: Boolean? = null,
    val dynarec_safeflags: Int? = null,
    val dynarec_strongmem: Int? = null,
    val dynarec_bigblock: Int? = null,
    val log_level: String? = null,
    val extra_env: Map<String, String> = emptyMap()
)

data class Box64EnvironmentConfigData(
    val enabled: Boolean = true,
    val binary_version: String = "0.4.4",
    val dynarec_enabled: Boolean = true,
    val dynarec_safeflags: Int = 1,
    val dynarec_strongmem: Int = 1,
    val dynarec_bigblock: Int = 1,
    val log_level: String = "errors",
    val dynarec_log_level: Int = 0,
    val show_segv: Boolean = true,
    val show_bt: Boolean = true,
    val library_search_paths: List<String> = listOf(
        "/usr/local/lib/box64-x86_64-linux-gnu",
        "/usr/lib/x86_64-linux-gnu",
        "/lib/x86_64-linux-gnu"
    ),
    val env_vars: Map<String, String> = emptyMap(),
    val app_overrides: Map<String, Box64AppOverrideData> = emptyMap()
)

data class Box64ExecutionOutcomeData(
    val environment_id: String = "",
    val binary_path: String = "",
    val route: String = "unsupported",
    val architecture: String = "unknown",
    val success: Boolean = false,
    val exit_code: Int? = null,
    val terminating_signal: Int? = null,
    val error_kind: String = "none",
    val error_explanation: String? = null,
    val stdout: String = "",
    val stderr: String = "",
    val missing_libraries: List<String> = emptyList(),
    val duration_ms: Long = 0L,
    val timestamp_epoch_ms: Long = 0L,
    val error: String? = null
)

data class Box64ExecutionRecordData(
    val timestamp_epoch_ms: Long = 0L,
    val environment_id: String = "",
    val binary_path: String = "",
    val route: String = "unsupported",
    val architecture: String = "unknown",
    val success: Boolean = false,
    val exit_code: Int? = null,
    val terminating_signal: Int? = null,
    val error_kind: String = "none",
    val summary: String = "",
    val duration_ms: Long = 0L
)

data class CompatibilityTestCaseData(
    val id: String = "",
    val category: String = "",
    val title: String = "",
    val status: String = "device_test_required",
    val detail: String = ""
)

data class CompatibilitySuiteReportData(
    val box64_version: String = "0.4.4",
    val box64_installed: Boolean = false,
    val host_arch: String = "aarch64",
    val passed: Int = 0,
    val failed: Int = 0,
    val device_required: Int = 0,
    val cases: List<CompatibilityTestCaseData> = emptyList()
)

// -----------------------------------------------------------------------------
// Phase 4 — Complete Windows Compatibility Engine Models
// -----------------------------------------------------------------------------

data class WineInstallationStatusData(
    val installed: Boolean = false,
    val binary_path: String? = null,
    val detected_version: String? = null,
    val pinned_version: String = "11.0",
    val license: String = "LGPL-2.1-or-later",
    val architecture_strategy: String = "x86_64 Wine 11.0 (pure-64-bit WoW64) via Box64",
    val wow64_enabled: Boolean = true,
    val box86_required: Boolean = false,
    val diagnostics: List<String> = emptyList()
)

data class WinePrefixMetadataData(
    val id: String = "",
    val display_name: String = "",
    val environment_id: String = "",
    val wine_version: String = "11.0",
    val windows_version: String = "win10",
    val arch: String = "WoW64",
    val prefix_path: String = "",
    val env_vars: Map<String, String> = emptyMap(),
    val dll_overrides: Map<String, String> = emptyMap(),
    val graphics_backend: String = "software",
    val dxvk_enabled: Boolean = false,
    val vkd3d_enabled: Boolean = false,
    val installed_apps: List<String> = emptyList(),
    val dependencies: List<String> = emptyList(),
    val created_at_epoch_ms: Long = 0L,
    val last_used_at_epoch_ms: Long = 0L,
    val error: String? = null
)

data class BackendCapabilityReportData(
    val kind: String = "software",
    val display_name: String = "",
    val pinned_version: String = "26.3.1",
    val license: String = "MIT",
    val stage: String = "not_installed",
    val supports_opengl: Boolean = true,
    val supports_vulkan: Boolean = false,
    val vulkan_api_version: String? = null,
    val dxvk_compatible: Boolean = false,
    val vkd3d_compatible: Boolean = false,
    val hardware_accelerated: Boolean = false,
    val detected_gpu: String? = null,
    val icd_path: String? = null,
    val verified_extensions: List<String> = emptyList(),
    val warnings: List<String> = emptyList(),
    val diagnostics: List<String> = emptyList()
)

data class DxTranslationStatusData(
    val dxvk_pinned_version: String = "2.7.1",
    val dxvk_license: String = "zlib",
    val dxvk_staged: Boolean = false,
    val dxvk_eligible: Boolean = false,
    val dxvk_fallback_reason: String? = null,
    val vkd3d_pinned_version: String = "3.1",
    val vkd3d_license: String = "LGPL-2.1",
    val vkd3d_staged: Boolean = false,
    val vkd3d_eligible: Boolean = false,
    val vkd3d_missing_extensions: List<String> = emptyList(),
    val vkd3d_fallback_reason: String? = null,
    val effective_choice: String = "wine_d3d_open_gl"
)

data class GraphicsBackendConfigData(
    val preferred_backend: String = "software",
    val allow_software_fallback: Boolean = true,
    val dxvk_enabled: Boolean = false,
    val vkd3d_proton_enabled: Boolean = false,
    val enable_hud: Boolean = false,
    val turnip_debug_flags: String = "noconform",
    val enable_zink_gl_override: Boolean = true,
    val virgl_socket_path: String = "/tmp/.virgl_test",
    val shader_cache_dir: String = "/home/user/.cache/androidpc-shaders"
)

data class GraphicsStackOverviewData(
    val active_config: GraphicsBackendConfigData = GraphicsBackendConfigData(),
    val effective_backend: String = "software",
    val fallback_engaged: Boolean = false,
    val fallback_reason: String? = null,
    val backends: List<BackendCapabilityReportData> = emptyList(),
    val dx_translation: DxTranslationStatusData = DxTranslationStatusData()
)

data class WindowsAppProfileData(
    val app_id: String = "",
    val name: String = "",
    val executable_path: String = "",
    val install_dir: String = "",
    val prefix_id: String = "",
    val environment_id: String = "",
    val wine_version: String = "11.0",
    val graphics_backend: String = "software",
    val resolution_width: Int = 1280,
    val resolution_height: Int = 720,
    val launch_args: List<String> = emptyList(),
    val env_vars: Map<String, String> = emptyMap(),
    val compatibility_notes: String = "",
    val created_at_epoch_ms: Long = 0L,
    val last_launched_at_epoch_ms: Long? = null,
    val shortcut_created: Boolean = false,
    val error: String? = null
)

data class WindowsAppLaunchResultData(
    val app_id: String = "",
    val prefix_id: String = "",
    val spawned_pid: Long? = null,
    val effective_graphics_backend: String = "software",
    val dx_choice: String = "wine_d3d_open_gl",
    val warnings: List<String> = emptyList(),
    val error: String? = null
)

data class TrackedWindowsProcessData(
    val app_id: String = "",
    val app_name: String = "",
    val prefix_id: String = "",
    val environment_id: String = "",
    val wine_pid: Long? = null,
    val box64_pid: Long? = null,
    val state: String = "launching",
    val memory_rss_bytes: Long? = null,
    val exit_code: Int? = null,
    val exit_reason: String? = null,
    val started_at_epoch_ms: Long = 0L
)

data class DependencyProfileData(
    val id: String = "",
    val display_name: String = "",
    val version: String = "",
    val category: String = "",
    val license: String = "",
    val mechanism: String = "verified_open_source_artifact",
    val official_source_url: String = "",
    val expected_dlls: List<String> = emptyList(),
    val conflicts_with: List<String> = emptyList(),
    val recovery_hint: String = ""
)

data class DependencyConflictData(
    val profile_a: String = "",
    val profile_b: String = "",
    val reason: String = "",
    val recovery_action: String = ""
)

data class PrefixDependencyDiagnosisData(
    val prefix_id: String = "",
    val present_dlls: List<String> = emptyList(),
    val missing_dlls: List<String> = emptyList(),
    val recommended_profiles: List<String> = emptyList(),
    val conflicts: List<DependencyConflictData> = emptyList(),
    val notes: List<String> = emptyList()
)

data class WindowsCompatibilityOverviewData(
    val environment_id: String = "",
    val box64: Box64InstallationStatusData = Box64InstallationStatusData(),
    val wine: WineInstallationStatusData = WineInstallationStatusData(),
    val graphics: GraphicsStackOverviewData = GraphicsStackOverviewData(),
    val prefix_count: Int = 0,
    val installed_apps_count: Int = 0,
    val active_processes_count: Int = 0,
    val runtime_errors: List<String> = emptyList()
)

// -----------------------------------------------------------------------------
// Phase 5 — Full Virtual Machine Engine (QEMU 11.1.3) Models
// -----------------------------------------------------------------------------

data class AvfCapabilitiesData(
    val ordinary_app_accessible: Boolean = false,
    val permission_required: String = "android.permission.MANAGE_VIRTUAL_MACHINE (@SystemApi)",
    val notes: String = ""
)

data class QemuEngineCapabilitiesData(
    val pinned_qemu_version: String = "11.1.3",
    val pinned_edk2_version: String = "edk2-stable202608",
    val pinned_libslirp_version: String = "4.9.7",
    val qemu_aarch64_installed: Boolean = false,
    val qemu_x86_64_installed: Boolean = false,
    val qemu_img_installed: Boolean = false,
    val aavmf_firmware_installed: Boolean = false,
    val ovmf_firmware_installed: Boolean = false,
    val kvm: KvmCapabilitiesData = KvmCapabilitiesData(),
    val avf: AvfCapabilitiesData = AvfCapabilitiesData(),
    val networking_notes: String = "",
    val security_boundary_note: String = ""
)

data class VmCpuConfigData(
    val cores: Int = 2,
    val threads: Int = 1,
    val model: String = "cortex-a72",
    val tcg_tb_size_mb: Int = 128
)

data class VmFirmwareConfigData(
    val kind: String = "aavmf_uefi",
    val code_path: String? = null,
    val vars_path: String? = null,
    val secure_boot: Boolean = false
)

data class VmStorageDeviceData(
    val disk_id: String = "disk0",
    val path: String = "",
    val format: String = "qcow2",
    val bus: String = "virtio_blk",
    val size_mb: Long = 2048L,
    val read_only: Boolean = false,
    val boot_index: Int? = 1
)

data class VmPortForwardRuleData(
    val protocol: String = "tcp",
    val host_port: Int = 2222,
    val guest_port: Int = 22,
    val bind_loopback_only: Boolean = true
)

data class VmNetworkConfigData(
    val mode: String = "user_slirp",
    val nic_model: String = "virtio-net-pci",
    val guest_cidr: String = "10.0.2.0/24",
    val dns_server: String = "10.0.2.3",
    val port_forwards: List<VmPortForwardRuleData> = listOf(VmPortForwardRuleData())
)

data class VmDisplayConfigData(
    val backend: String = "vnc_localhost",
    val gpu_device: String = "virtio-gpu-pci",
    val vnc_display: Int = 10,
    val width: Int = 1280,
    val height: Int = 720,
    val enable_usb_tablet: Boolean = true,
    val enable_clipboard_note: String = ""
) {
    fun vncPort(): Int = 5900 + vnc_display
}

data class VmAudioConfigData(
    val backend: String = "disabled",
    val device_model: String = "intel-hda",
    val pulse_server: String = "tcp:127.0.0.1:4713",
    val limitation_note: String = ""
)

data class VmBootConfigData(
    val boot_order: String = "cdn",
    val iso_path: String? = null,
    val direct_kernel: String? = null,
    val direct_initrd: String? = null,
    val kernel_cmdline: String? = null
)

data class VmProfileData(
    val id: String = "",
    val name: String = "",
    val arch: String = "aarch64",
    val machine_type: String = "virt,gic-version=3",
    val cpu: VmCpuConfigData = VmCpuConfigData(),
    val memory_mb: Int = 1024,
    val accelerator_preference: String = "tcg_multi",
    val firmware: VmFirmwareConfigData = VmFirmwareConfigData(),
    val disks: List<VmStorageDeviceData> = emptyList(),
    val network: VmNetworkConfigData = VmNetworkConfigData(),
    val display: VmDisplayConfigData = VmDisplayConfigData(),
    val audio: VmAudioConfigData = VmAudioConfigData(),
    val boot: VmBootConfigData = VmBootConfigData(),
    val qmp_port: Int = 4444,
    val qemu_version: String = "11.1.3",
    val created_at_epoch_ms: Long = 0L,
    val last_used_at_epoch_ms: Long? = null,
    val state: String = "Stopped",
    val notes: String = ""
)

data class VmProcessTelemetryData(
    val pid: Long = 0L,
    val start_time_ticks: Long? = null,
    val memory_rss_bytes: Long? = null,
    val cpu_time_ms: Long? = null,
    val alive: Boolean = false
)

data class VmRuntimeSummaryData(
    val profile: VmProfileData = VmProfileData(),
    val effective_accelerator: String = "tcg",
    val execution_model_note: String = "",
    val pid: Long? = null,
    val telemetry: VmProcessTelemetryData? = null,
    val storage_used_bytes: Long = 0L,
    val snapshot_count: Int = 0,
    val last_exit_reason: String? = null,
    val startup_error: String? = null,
    val error: String? = null
)

data class VmDiskInspectionData(
    val path: String = "",
    val exists: Boolean = false,
    val detected_format: String = "unknown",
    val virtual_size_bytes: Long = 0L,
    val actual_disk_bytes: Long = 0L,
    val qcow2_version: Int? = null,
    val healthy: Boolean = false,
    val diagnostics: List<String> = emptyList()
)

data class VmSnapshotMetadataData(
    val snapshot_id: String = "",
    val vm_id: String = "",
    val name: String = "",
    val description: String = "",
    val disk_format: String = "qcow2",
    val mechanism: String = "offline_verified_copy",
    val created_at_epoch_ms: Long = 0L,
    val disk_sha256: String? = null,
    val backup_disk_path: String? = null,
    val error: String? = null
)

data class VmGuestImageEntryData(
    val id: String = "",
    val display_name: String = "",
    val distribution: String = "",
    val version: String = "",
    val arch: String = "aarch64",
    val media_type: String = "iso",
    val recommended_firmware: String = "aavmf_uefi",
    val min_ram_mb: Int = 512,
    val min_disk_mb: Long = 1024L,
    val official_download_url: String = "",
    val checksum_url: String = "",
    val license: String = "",
    val redistribution_policy: String = "",
    val notes: String = ""
)



