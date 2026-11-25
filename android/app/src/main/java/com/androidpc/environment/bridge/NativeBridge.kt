package com.androidpc.environment.bridge

import android.content.Context
import android.util.Log
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

object NativeBridge {
    private const val TAG = "NativeBridge"
    private const val LIB_NAME = "androidpc_engine"

    private val gson = Gson()
    var isLoaded = false
        private set

    init {
        try {
            System.loadLibrary(LIB_NAME)
            isLoaded = true
            Log.i(TAG, "Native library '$LIB_NAME' loaded successfully")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Failed to load native library '$LIB_NAME': ${e.message}", e)
            isLoaded = false
        }
    }

    // External JNI declarations
    private external fun nativeInitEngine(configJson: String): Int
    private external fun nativeGetEngineVersion(): String
    private external fun nativeGetEngineStatus(): String
    private external fun nativeRunHealthCheck(): String
    private external fun nativeGetDiagnostics(): String
    private external fun nativeGetRecentLogs(limit: Int): String
    private external fun nativeStartDisplayServer(displayNum: Int, width: Int, height: Int, dpi: Int, port: Int): String
    private external fun nativeStopDisplayServer(): Int
    private external fun nativeIsDisplayRunning(): Boolean
    private external fun nativeCreateTerminal(sessionId: String, cols: Int, rows: Int): Int
    private external fun nativeWriteTerminal(sessionId: String, data: ByteArray): Int
    private external fun nativeReadTerminal(sessionId: String, maxLen: Int): ByteArray
    private external fun nativeResizeTerminal(sessionId: String, cols: Int, rows: Int): Int
    private external fun nativeCloseTerminal(sessionId: String): Int
    private external fun nativeRfbBuildPointerEvent(buttonMask: Int, x: Int, y: Int): ByteArray
    private external fun nativeRfbBuildKeyEvent(downFlag: Int, keySym: Long): ByteArray
    private external fun nativeProvisionEnvironment(envId: String, distro: String): Int
    private external fun nativeExecuteGuestCommand(envId: String, commandJson: String): Int
    private external fun nativeStopEnvironment(envId: String): Int
    private external fun nativeGetEnvironmentState(envId: String): String
    private external fun nativeListEnvironments(): String
    private external fun nativeGetArtifactInfo(distro: String): String
    private external fun nativeConnectDisplayClient(host: String, port: Int): Long
    private external fun nativeGetFramebuffer(pixelsOut: ByteArray): Int
    private external fun nativeSendPointer(buttonMask: Int, x: Int, y: Int): Int
    private external fun nativeSendKey(downFlag: Int, keySym: Long): Int
    private external fun nativeDisconnectDisplayClient(): Int
    private external fun nativeInitWinePrefix(envId: String, prefixPath: String, arch: String): Int
    private external fun nativeBuildWineCommand(envId: String, exePath: String, argsJson: String): String
    private external fun nativeKillWineSession(envId: String): Int
    private external fun nativeGetWineStatus(envId: String): String
    private external fun nativeStartEnvironment(envId: String): Int
    private external fun nativeDeleteEnvironment(envId: String): Int
    private external fun nativeVerifyAndInstallArtifact(partPath: String, destFilename: String, expectedSha256: String): Int
    private external fun nativeListArtifacts(): String
    private external fun nativeStartAudioSink(port: Int): Int
    private external fun nativeStopAudioSink(): Int
    private external fun nativeReadAudioPcm(pcmOut: ByteArray): Int
    private external fun nativeGetAudioStatus(): String
    private external fun nativeLaunchWineApp(envId: String, exePath: String, argsJson: String): String
    private external fun nativeVerifyArtifact(path: String, expectedSha256: String): Boolean
    private external fun nativeExtractRootfs(archivePath: String, targetDir: String): Long
    private external fun nativeProvisionEnvironmentWithMemory(envId: String, distro: String, memoryMb: Int): Int
    private external fun nativeProbeKvmSupport(): String
    private external fun nativeCreateVm(vmId: String, arch: String, memoryMb: Int, cpus: Int, diskSizeMb: Int): Int
    private external fun nativeStartVm(vmId: String): Int
    private external fun nativeStopVm(vmId: String): Int
    private external fun nativeDeleteVm(vmId: String): Int
    private external fun nativeListVms(): String
    private external fun nativeCreateTerminalForEnv(sessionId: String, envId: String, cols: Int, rows: Int): Int

    suspend fun initEngine(context: Context): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.failure(IllegalStateException("Native engine library not loaded"))
        }

        try {
            val baseData = context.filesDir.absolutePath
            val artifacts = AssetInstaller.installBundledAssets(context).absolutePath
            val tmp = File(context.cacheDir, "tmp").apply { mkdirs() }.absolutePath

            val config = EngineConfigData(
                base_data_dir = baseData,
                runtime_artifacts_dir = artifacts,
                temporary_dir = tmp,
                native_lib_dir = context.applicationInfo.nativeLibraryDir,
                debug_mode = true,
                log_level = "info",
                max_concurrent_processes = 32
            )

            val json = gson.toJson(config)
            val code = nativeInitEngine(json)
            if (code == 0) {
                Log.i(TAG, "Rust engine initialized successfully with code 0")
                Result.success(code)
            } else {
                Log.e(TAG, "Rust engine initialization failed with code $code")
                Result.failure(RuntimeException("Native init error code: $code"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "Exception during engine init: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun getEngineVersion(): String = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext "Engine unlinked"
        try {
            nativeGetEngineVersion()
        } catch (e: Exception) {
            Log.e(TAG, "nativeGetEngineVersion failed: ${e.message}", e)
            "Error: ${e.message}"
        }
    }

    suspend fun getEngineStatus(): Result<EngineStatusData> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.failure(IllegalStateException("Native library not loaded"))
        }
        try {
            val raw = nativeGetEngineStatus()
            val parsed = gson.fromJson(raw, EngineStatusData::class.java)
            Result.success(parsed)
        } catch (e: Exception) {
            Log.e(TAG, "nativeGetEngineStatus failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun runHealthCheck(): Result<HealthReportData> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.failure(IllegalStateException("Native library not loaded"))
        }
        try {
            val raw = nativeRunHealthCheck()
            val parsed = gson.fromJson(raw, HealthReportData::class.java)
            Result.success(parsed)
        } catch (e: Exception) {
            Log.e(TAG, "nativeRunHealthCheck failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun getDiagnostics(): Result<SystemDiagnosticsData> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.failure(IllegalStateException("Native library not loaded"))
        }
        try {
            val raw = nativeGetDiagnostics()
            val parsed = gson.fromJson(raw, SystemDiagnosticsData::class.java)
            Result.success(parsed)
        } catch (e: Exception) {
            Log.e(TAG, "nativeGetDiagnostics failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun getRecentLogs(limit: Int = 50): Result<List<LogEntryData>> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.success(emptyList())
        }
        try {
            val raw = nativeGetRecentLogs(limit)
            val listType = object : TypeToken<List<LogEntryData>>() {}.type
            val parsed: List<LogEntryData> = gson.fromJson(raw, listType)
            Result.success(parsed)
        } catch (e: Exception) {
            Log.e(TAG, "nativeGetRecentLogs failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun startDisplayServer(
        displayNum: Int = 1,
        width: Int = 1920,
        height: Int = 1080,
        dpi: Int = 160,
        port: Int = 5901
    ): Result<DisplayEndpointData> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        }
        try {
            val raw = nativeStartDisplayServer(displayNum, width, height, dpi, port)
            if (raw.contains("\"error\"")) {
                Result.failure(RuntimeException(raw))
            } else {
                val parsed = gson.fromJson(raw, DisplayEndpointData::class.java)
                Result.success(parsed)
            }
        } catch (e: Exception) {
            Log.e(TAG, "nativeStartDisplayServer failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun stopDisplayServer(): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeStopDisplayServer()
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Stop display error $code"))
        } catch (e: Exception) {
            Log.e(TAG, "nativeStopDisplayServer failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    fun isDisplayRunning(): Boolean {
        if (!isLoaded) return false
        return try {
            nativeIsDisplayRunning()
        } catch (e: Exception) {
            false
        }
    }

    suspend fun createTerminal(sessionId: String, cols: Int = 80, rows: Int = 24): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val fd = nativeCreateTerminal(sessionId, cols, rows)
            if (fd >= 0) Result.success(fd) else Result.failure(RuntimeException("Create terminal error $fd"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun writeTerminal(sessionId: String, data: ByteArray): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val written = nativeWriteTerminal(sessionId, data)
            if (written >= 0) Result.success(written) else Result.failure(RuntimeException("Write terminal error $written"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun readTerminal(sessionId: String, maxLen: Int = 4096): Result<ByteArray> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val bytes = nativeReadTerminal(sessionId, maxLen)
            Result.success(bytes)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun resizeTerminal(sessionId: String, cols: Int, rows: Int): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeResizeTerminal(sessionId, cols, rows)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Resize terminal error $code"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun closeTerminal(sessionId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeCloseTerminal(sessionId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Close terminal error $code"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun buildPointerEvent(buttonMask: Int, x: Int, y: Int): ByteArray {
        if (!isLoaded) return ByteArray(0)
        return try {
            nativeRfbBuildPointerEvent(buttonMask, x, y)
        } catch (e: Exception) {
            ByteArray(0)
        }
    }

    fun buildKeyEvent(downFlag: Int, keySym: Long): ByteArray {
        if (!isLoaded) return ByteArray(0)
        return try {
            nativeRfbBuildKeyEvent(downFlag, keySym)
        } catch (e: Exception) {
            ByteArray(0)
        }
    }

    suspend fun provisionEnvironment(envId: String, distro: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeProvisionEnvironment(envId, distro)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Provision error code $code"))
        } catch (e: Exception) {
            Log.e(TAG, "provisionEnvironment failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun executeGuestCommand(envId: String, command: List<String>): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val json = gson.toJson(command)
            val pid = nativeExecuteGuestCommand(envId, json)
            if (pid > 0) Result.success(pid) else Result.failure(RuntimeException("Guest exec error code $pid"))
        } catch (e: Exception) {
            Log.e(TAG, "executeGuestCommand failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun stopEnvironment(envId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeStopEnvironment(envId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Stop env error code $code"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getEnvironmentState(envId: String): String = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext "EngineNotLoaded"
        try {
            nativeGetEnvironmentState(envId)
        } catch (e: Exception) {
            "Error: ${e.message}"
        }
    }

    suspend fun listEnvironments(): Result<List<EnvironmentConfigData>> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.success(emptyList())
        try {
            val raw = nativeListEnvironments()
            val listType = object : TypeToken<List<EnvironmentConfigData>>() {}.type
            val parsed: List<EnvironmentConfigData> = gson.fromJson(raw, listType)
            Result.success(parsed)
        } catch (e: Exception) {
            Log.e(TAG, "listEnvironments failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun getArtifactInfo(distro: String): Result<ArtifactMetadataData> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val raw = nativeGetArtifactInfo(distro)
            val parsed = gson.fromJson(raw, ArtifactMetadataData::class.java)
            Result.success(parsed)
        } catch (e: Exception) {
            Log.e(TAG, "getArtifactInfo failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun connectDisplayClient(host: String = "127.0.0.1", port: Int = 5901): Pair<Int, Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Pair(1280, 720)
        try {
            val packed = nativeConnectDisplayClient(host, port)
            if (packed == 0L) {
                Pair(1280, 720)
            } else {
                val width = (packed ushr 32).toInt()
                val height = (packed and 0xffffffffL).toInt()
                Pair(width, height)
            }
        } catch (e: Exception) {
            Pair(1280, 720)
        }
    }

    fun getFramebuffer(pixelsOut: ByteArray): Int {
        if (!isLoaded) return -1
        return try {
            nativeGetFramebuffer(pixelsOut)
        } catch (e: Exception) {
            -1
        }
    }

    fun sendPointer(buttonMask: Int, x: Int, y: Int): Int {
        if (!isLoaded) return -1
        return try {
            nativeSendPointer(buttonMask, x, y)
        } catch (e: Exception) {
            -1
        }
    }

    fun sendKey(downFlag: Int, keySym: Long): Int {
        if (!isLoaded) return -1
        return try {
            nativeSendKey(downFlag, keySym)
        } catch (e: Exception) {
            -1
        }
    }

    fun disconnectDisplayClient(): Int {
        if (!isLoaded) return 0
        return try {
            nativeDisconnectDisplayClient()
        } catch (e: Exception) {
            -1
        }
    }

    suspend fun initWinePrefix(
        envId: String,
        prefixPath: String = "/home/user/.wine",
        arch: String = "win64"
    ): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeInitWinePrefix(envId, prefixPath, arch)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Wine init error $code"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun buildWineLaunchCommand(
        envId: String,
        exePath: String,
        args: List<String> = emptyList()
    ): Result<ProcessCommandData> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val argsJson = gson.toJson(args)
            val raw = nativeBuildWineCommand(envId, exePath, argsJson)
            val cmd = gson.fromJson(raw, ProcessCommandData::class.java)
            Result.success(cmd)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun killWineSession(envId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeKillWineSession(envId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Wine kill error $code"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getWineStatus(envId: String): Result<WineStatusData> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val raw = nativeGetWineStatus(envId)
            val parsed = gson.fromJson(raw, WineStatusData::class.java)
            Result.success(parsed)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun startEnvironment(envId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeStartEnvironment(envId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Start environment failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun deleteEnvironment(envId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeDeleteEnvironment(envId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Delete environment failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun listArtifacts(): Result<List<ArtifactMetadataData>> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.success(emptyList())
        try {
            val raw = nativeListArtifacts()
            val listType = object : TypeToken<List<ArtifactMetadataData>>() {}.type
            val parsed: List<ArtifactMetadataData> = gson.fromJson(raw, listType)
            Result.success(parsed)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun verifyAndInstallArtifact(partPath: String, destFilename: String, expectedSha256: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeVerifyAndInstallArtifact(partPath, destFilename, expectedSha256)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Install artifact failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun extractRootfs(archivePath: String, targetDir: String): Result<Long> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val count = nativeExtractRootfs(archivePath, targetDir)
            if (count >= 0) Result.success(count) else Result.failure(RuntimeException("Extract rootfs failed ($count)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun launchWineApp(
        envId: String,
        exePath: String,
        args: List<String> = emptyList()
    ): Result<ProcessCommandData> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val argsJson = gson.toJson(args)
            val raw = nativeLaunchWineApp(envId, exePath, argsJson)
            val parsed = gson.fromJson(raw, ProcessCommandData::class.java)
            if (parsed.error != null) {
                Result.failure(RuntimeException(parsed.error))
            } else {
                Result.success(parsed)
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun startAudioSink(port: Int = 4713): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val bound = nativeStartAudioSink(port)
            if (bound > 0) Result.success(bound) else Result.failure(RuntimeException("Start audio sink failed ($bound)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun stopAudioSink(): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeStopAudioSink()
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Stop audio sink failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun readAudioPcm(pcmOut: ByteArray): Int {
        if (!isLoaded) return 0
        return try {
            nativeReadAudioPcm(pcmOut)
        } catch (e: Exception) {
            0
        }
    }

    suspend fun getAudioStatus(): Result<AudioStatusData> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val raw = nativeGetAudioStatus()
            Result.success(gson.fromJson(raw, AudioStatusData::class.java))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun provisionEnvironmentWithMemory(envId: String, distro: String, memoryMb: Int): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeProvisionEnvironmentWithMemory(envId, distro, memoryMb)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Provision failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun verifyArtifact(path: String, expectedSha256: String): Result<Boolean> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            Result.success(nativeVerifyArtifact(path, expectedSha256))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun probeKvmSupport(): Result<KvmCapabilitiesData> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val raw = nativeProbeKvmSupport()
            Result.success(gson.fromJson(raw, KvmCapabilitiesData::class.java))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun createVm(vmId: String, arch: String, memoryMb: Int, cpus: Int, diskSizeMb: Int): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeCreateVm(vmId, arch, memoryMb, cpus, diskSizeMb)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Create VM failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun startVm(vmId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeStartVm(vmId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Start VM failed ($code) — ensure QEMU binary is installed"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun stopVm(vmId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeStopVm(vmId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Stop VM failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun deleteVm(vmId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeDeleteVm(vmId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Delete VM failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun listVms(): Result<List<VmStatusData>> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val raw = nativeListVms()
            val listType = object : TypeToken<List<VmStatusData>>() {}.type
            val parsed: List<VmStatusData> = gson.fromJson(raw, listType)
            Result.success(parsed)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun createTerminalForEnv(sessionId: String, envId: String, cols: Int = 80, rows: Int = 24): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val fd = nativeCreateTerminalForEnv(sessionId, envId, cols, rows)
            if (fd >= 0) Result.success(fd) else Result.failure(RuntimeException("Create terminal failed ($fd)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
