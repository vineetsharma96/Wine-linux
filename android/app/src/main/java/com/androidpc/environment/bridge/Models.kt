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
