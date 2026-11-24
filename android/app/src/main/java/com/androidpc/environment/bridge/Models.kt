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
    val debug_mode: Boolean,
    val log_level: String,
    val max_concurrent_processes: Int
)

data class DisplayEndpointData(
    val display_type: String = "Xvnc",
    val address: String = "127.0.0.1",
    val port: Int = 5901,
    val auth_required: Boolean = false
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
    val distribution: String = "",
    val rootfs_directory: String = "",
    val default_username: String = "user",
    val memory_limit_mb: Long? = null
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
    val cwd: String? = null
)
