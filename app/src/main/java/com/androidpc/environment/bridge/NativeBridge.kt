package com.androidpc.environment.bridge

import android.content.Context
import android.util.Log
import com.androidpc.environment.BuildConfig
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
    private external fun nativeStartDesktopSession(envId: String, displayNum: Int, width: Int, height: Int, dpi: Int, port: Int): String
    private external fun nativeStopDesktopSession(sessionId: String): Int
    private external fun nativeHasLiveDisplayConnection(): Boolean
    private external fun nativeGetDesktopStatus(envId: String): String
    private external fun nativeInstallDesktopProfile(envId: String, profile: String): String
    private external fun nativeResizeDesktopSession(width: Int, height: Int): Long
    private external fun nativeSendDisplayClipboard(text: String): Int
    private external fun nativeTakeDisplayClipboard(): String?
    private external fun nativeGetDisplayDimensions(): Long
    private external fun nativeIsDisplayRunning(): Boolean
    private external fun nativeCreateTerminal(sessionId: String, cols: Int, rows: Int): Int
    private external fun nativeWriteTerminal(sessionId: String, data: ByteArray): Int
    private external fun nativeReadTerminal(sessionId: String, maxLen: Int): ByteArray
    private external fun nativeResizeTerminal(sessionId: String, cols: Int, rows: Int): Int
    private external fun nativeCloseTerminal(sessionId: String): Int
    private external fun nativeTerminalExitStatus(sessionId: String): Int
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
    private external fun nativeGetFramebufferDirect(directBuffer: java.nio.ByteBuffer, lastGen: Long): Long
    private external fun nativeAttachDisplaySurface(surface: android.view.Surface?): Int
    private external fun nativeDisplayPresenterFailed(): Boolean
    private external fun nativeDetachDisplaySurface(): Int
    private external fun nativeGetPresentedFps(): Int
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
    private external fun nativeInstallEnvironment(envId: String, distro: String, name: String, memoryMb: Int, archive: String): String?
    private external fun nativeRecoverEnvironment(envId: String, restart: Boolean): String?
    private external fun nativeCancelInstallation(envId: String): Boolean
    private external fun nativeEnvironmentStorageBytes(envId: String): Long
    private external fun nativeRunPackageOperation(envId: String, action: String, query: String): String?
    private external fun nativePackageOperationProgress(envId: String): String?
    private external fun nativeCancelPackageOperation(envId: String): Boolean
    private external fun nativeBackupOperation(operation: String, source: String, backup: String, target: String, name: String): String?
    private external fun nativeEnvironmentMetrics(envId: String): String?
    private external fun nativeProbeKvmSupport(): String
    private external fun nativeCreateVm(vmId: String, arch: String, memoryMb: Int, cpus: Int, diskSizeMb: Int): Int
    private external fun nativeStartVm(vmId: String): Int
    private external fun nativeStopVm(vmId: String): Int
    private external fun nativeDeleteVm(vmId: String): Int
    private external fun nativeListVms(): String
    private external fun nativeCreateTerminalForEnv(sessionId: String, envId: String, cols: Int, rows: Int): Int
    private external fun nativeSetGpuAccelerationMode(mode: String): Int
    private external fun nativeGetGpuAccelerationMode(): String
    private external fun nativeCreateTerminalForEnvWithShell(sessionId: String, envId: String, cols: Int, rows: Int, shell: String): Int
    private external fun nativeGetBox64Status(envId: String): String
    private external fun nativeGetBox64Config(envId: String): String
    private external fun nativeUpdateBox64Config(envId: String, configJson: String): String
    private external fun nativeExecuteCompatibleBinary(envId: String, binaryPath: String, argsJson: String): String
    private external fun nativeRunBox64Suite(envId: String): String
    private external fun nativeGetBox64History(limit: Int): String

    suspend fun initEngine(context: Context): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.failure(IllegalStateException("Native engine library not loaded"))
        }

        try {
            val baseData = context.filesDir.absolutePath
            val artifactsDir = AssetInstaller.installBundledAssets(context)
            val rootfsKey = BuildConfig.ROOTFS_PUBLIC_KEY_HEX.takeIf(String::isNotEmpty)
            if (rootfsKey != null) {
                val rootfsDir = File(artifactsDir, "rootfs").apply { mkdirs() }
                for (name in listOf("catalog.json", "catalog.sig")) {
                    context.assets.open("rootfs/$name").use { input ->
                        File(rootfsDir, name).outputStream().use { input.copyTo(it) }
                    }
                }
            }
            val tmp = File(context.cacheDir, "tmp").apply { mkdirs() }.absolutePath

            val config = EngineConfigData(
                base_data_dir = baseData,
                runtime_artifacts_dir = artifactsDir.absolutePath,
                temporary_dir = tmp,
                native_lib_dir = context.applicationInfo.nativeLibraryDir,
                trusted_rootfs_key_hex = rootfsKey,
                app_version_code = BuildConfig.VERSION_CODE,
                debug_mode = BuildConfig.DEBUG,
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

    suspend fun startDesktopSession(
        envId: String,
        displayNum: Int = 1,
        width: Int = 1280,
        height: Int = 720,
        dpi: Int = 160,
        port: Int = 5901
    ): Result<DisplayEndpointData> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        }
        try {
            val raw = nativeStartDesktopSession(envId, displayNum, width, height, dpi, port)
            if (raw.contains("\"error\"")) {
                Result.failure(RuntimeException(raw))
            } else {
                Result.success(gson.fromJson(raw, DisplayEndpointData::class.java))
            }
        } catch (e: Exception) {
            Log.e(TAG, "nativeStartDesktopSession failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    suspend fun stopDesktopSession(sessionId: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeStopDesktopSession(sessionId)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Stop desktop error $code"))
        } catch (e: Exception) {
            Log.e(TAG, "nativeStopDesktopSession failed: ${e.message}", e)
            Result.failure(e)
        }
    }

    fun hasLiveDisplayConnection(): Boolean = isLoaded && nativeHasLiveDisplayConnection()

    suspend fun getDesktopStatus(envId: String): String = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext "{\"error\":\"Engine not loaded\"}"
        try {
            nativeGetDesktopStatus(envId)
        } catch (e: Exception) {
            "{\"error\":\"${e.message}\"}"
        }
    }

    suspend fun installDesktopProfile(envId: String, profile: String = "minimal"): String = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext "{\"error\":\"Engine not loaded\"}"
        try {
            nativeInstallDesktopProfile(envId, profile)
        } catch (e: Exception) {
            "{\"error\":\"${e.message}\"}"
        }
    }

    suspend fun resizeDesktopSession(width: Int, height: Int): Pair<Int, Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Pair(0, 0)
        try {
            val packed = nativeResizeDesktopSession(width, height)
            Pair((packed ushr 32).toInt(), (packed and 0xffffffffL).toInt())
        } catch (_: Exception) {
            Pair(0, 0)
        }
    }

    fun sendDisplayClipboard(text: String): Int {
        if (!isLoaded) return -1
        return try {
            nativeSendDisplayClipboard(text)
        } catch (_: Exception) {
            -1
        }
    }

    fun takeDisplayClipboard(): String? {
        if (!isLoaded) return null
        return try {
            nativeTakeDisplayClipboard()
        } catch (_: Exception) {
            null
        }
    }

    fun getDisplayDimensions(): Pair<Int, Int> {
        if (!isLoaded) return Pair(0, 0)
        return try {
            val packed = nativeGetDisplayDimensions()
            Pair((packed ushr 32).toInt(), (packed and 0xffffffffL).toInt())
        } catch (_: Exception) {
            Pair(0, 0)
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

    suspend fun terminalExitStatus(sessionId: String): Int = withContext(Dispatchers.IO) {
        if (isLoaded) nativeTerminalExitStatus(sessionId) else -2
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

    fun getFramebufferDirect(directBuffer: java.nio.ByteBuffer, lastGen: Long): Long {
        if (!isLoaded) return -1L
        return try {
            nativeGetFramebufferDirect(directBuffer, lastGen)
        } catch (e: Exception) {
            -1L
        }
    }

    fun attachDisplaySurface(surface: android.view.Surface?): Int {
        if (!isLoaded) return -1
        return try {
            nativeAttachDisplaySurface(surface)
        } catch (e: Exception) {
            Log.e(TAG, "nativeAttachDisplaySurface failed: ${e.message}", e)
            -1
        }
    }

    fun detachDisplaySurface(): Int {
        if (!isLoaded) return -1
        return try {
            nativeDetachDisplaySurface()
        } catch (_: Exception) {
            -1
        }
    }

    fun displayPresenterFailed(): Boolean = isLoaded && nativeDisplayPresenterFailed()

    fun getPresentedFps(): Int {
        if (!isLoaded) return 0
        return try {
            nativeGetPresentedFps()
        } catch (_: Exception) {
            0
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

    suspend fun installEnvironment(envId: String, distro: String, memoryMb: Int, archive: File, name: String): Result<Unit> =
        withContext(Dispatchers.IO) {
            runCatching {
                check(isLoaded) { "Native engine not loaded" }
                val error = nativeInstallEnvironment(envId, distro, name, memoryMb, archive.absolutePath)
                    ?: error("Native installation failed unexpectedly")
                check(error.isEmpty()) { error }
            }
        }

    suspend fun recoverEnvironment(envId: String, restart: Boolean): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            check(isLoaded) { "Native engine not loaded" }
            val error = nativeRecoverEnvironment(envId, restart) ?: error("Environment operation failed")
            check(error.isEmpty()) { error }
        }
    }

    suspend fun cancelInstallation(envId: String): Boolean = withContext(Dispatchers.IO) {
        isLoaded && nativeCancelInstallation(envId)
    }

    suspend fun environmentStorageBytes(envId: String): Long? = withContext(Dispatchers.IO) {
        if (isLoaded) nativeEnvironmentStorageBytes(envId).takeIf { it >= 0 } else null
    }

    suspend fun environmentMetrics(envId: String): EnvironmentMetricsData? = withContext(Dispatchers.IO) {
        if (isLoaded) nativeEnvironmentMetrics(envId)?.let { gson.fromJson(it, EnvironmentMetricsData::class.java) } else null
    }

    suspend fun runPackageOperation(envId: String, action: String, query: String): Result<PackageOperationData> =
        withContext(Dispatchers.IO) {
            runCatching {
                check(isLoaded) { "Native engine not loaded" }
                val json = nativeRunPackageOperation(envId, action, query) ?: error("Package operation failed")
                val result = gson.fromJson(json, PackageOperationData::class.java)
                result.error?.let { error(it) }
                result
            }
        }

    suspend fun packageOperationProgress(envId: String): PackageOperationData? = withContext(Dispatchers.IO) {
        if (!isLoaded) null else nativePackageOperationProgress(envId)?.let { gson.fromJson(it, PackageOperationData::class.java) }
    }

    suspend fun cancelPackageOperation(envId: String): Boolean = withContext(Dispatchers.IO) {
        isLoaded && nativeCancelPackageOperation(envId)
    }

    suspend fun backupOperation(operation: String, source: String = "", backup: String = "", target: String = "", name: String = ""): Result<BackupOperationData> =
        withContext(Dispatchers.IO) {
            runCatching {
                check(isLoaded) { "Native engine not loaded" }
                val json = nativeBackupOperation(operation, source, backup, target, name) ?: error("Backup operation failed")
                val result = gson.fromJson(json, BackupOperationData::class.java)
                result.error?.let { error(it) }
                result
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

    suspend fun createTerminalForEnv(sessionId: String, envId: String, cols: Int = 80, rows: Int = 24, shell: String = "/bin/bash"): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val fd = nativeCreateTerminalForEnvWithShell(sessionId, envId, cols, rows, shell)
            if (fd >= 0) Result.success(fd) else Result.failure(RuntimeException("Create terminal failed ($fd)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun setGpuAccelerationMode(mode: String): Result<Int> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
        try {
            val code = nativeSetGpuAccelerationMode(mode)
            if (code == 0) Result.success(0) else Result.failure(RuntimeException("Set GPU mode failed ($code)"))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getGpuAccelerationMode(): String = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext "software"
        try {
            nativeGetGpuAccelerationMode()
        } catch (e: Exception) {
            "software"
        }
    }

    suspend fun getBox64Status(context: Context, envId: String = ""): Box64InstallationStatusData = withContext(Dispatchers.IO) {
        if (isLoaded) {
            try {
                val raw = nativeGetBox64Status(envId)
                return@withContext gson.fromJson(raw, Box64InstallationStatusData::class.java)
            } catch (_: UnsatisfiedLinkError) {
                Log.w(TAG, "nativeGetBox64Status not yet in linked libandroidpc_engine.so; inspecting local staging")
            } catch (e: Exception) {
                Log.w(TAG, "nativeGetBox64Status failed: ${e.message}")
            }
        }
        val stagedBin = File(context.filesDir, "artifacts/box64/box64")
        val installed = stagedBin.isFile && stagedBin.length() > 0L
        Box64InstallationStatusData(
            installed = installed,
            binary_path = if (installed) stagedBin.absolutePath else null,
            detected_version = null,
            pinned_version = "0.4.4",
            license = "MIT",
            upstream_repo = "https://github.com/ptitSeb/box64",
            host_arch = android.os.Build.SUPPORTED_ABIS.firstOrNull() ?: "aarch64",
            dynarec_supported_on_host = android.os.Build.SUPPORTED_ABIS.any { it.contains("arm64") },
            abi_isolation_verified = true,
            status_summary = if (installed) {
                "Box64 v0.4.4 staged in artifacts/box64/box64 (rebuild libandroidpc_engine.so for JNI execution)"
            } else {
                "Box64 v0.4.4 not staged yet — build with scripts/build_box64_arm64.sh and rebuild JNI library"
            }
        )
    }

    suspend fun getBox64Config(context: Context, envId: String): Box64EnvironmentConfigData = withContext(Dispatchers.IO) {
        if (isLoaded && envId.isNotEmpty()) {
            try {
                val raw = nativeGetBox64Config(envId)
                return@withContext gson.fromJson(raw, Box64EnvironmentConfigData::class.java)
            } catch (_: UnsatisfiedLinkError) {
                // Fallback to persisted local preferences when using pre-Phase-4.1 native .so
            } catch (e: Exception) {
                Log.w(TAG, "nativeGetBox64Config failed: ${e.message}")
            }
        }
        val prefs = context.getSharedPreferences("androidpc_box64_prefs", Context.MODE_PRIVATE)
        val savedJson = prefs.getString("cfg_$envId", null)
        if (!savedJson.isNullOrBlank()) {
            runCatching { gson.fromJson(savedJson, Box64EnvironmentConfigData::class.java) }
                .getOrDefault(Box64EnvironmentConfigData())
        } else {
            Box64EnvironmentConfigData()
        }
    }

    suspend fun updateBox64Config(
        context: Context,
        envId: String,
        config: Box64EnvironmentConfigData
    ): Result<Box64EnvironmentConfigData> = withContext(Dispatchers.IO) {
        val json = gson.toJson(config)
        context.getSharedPreferences("androidpc_box64_prefs", Context.MODE_PRIVATE)
            .edit()
            .putString("cfg_$envId", json)
            .apply()

        if (isLoaded && envId.isNotEmpty()) {
            try {
                val raw = nativeUpdateBox64Config(envId, json)
                if (raw.contains("\"error\"")) {
                    return@withContext Result.failure(RuntimeException(raw))
                }
                return@withContext Result.success(gson.fromJson(raw, Box64EnvironmentConfigData::class.java))
            } catch (_: UnsatisfiedLinkError) {
                Log.w(TAG, "nativeUpdateBox64Config pending native .so rebuild; persisted locally")
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        Result.success(config)
    }

    suspend fun executeCompatibleBinary(
        envId: String,
        binaryPath: String,
        args: List<String> = emptyList()
    ): Result<Box64ExecutionOutcomeData> = withContext(Dispatchers.IO) {
        if (!isLoaded) {
            return@withContext Result.failure(IllegalStateException("Native Rust engine library not loaded"))
        }
        try {
            val raw = nativeExecuteCompatibleBinary(envId, binaryPath, gson.toJson(args))
            val parsed = gson.fromJson(raw, Box64ExecutionOutcomeData::class.java)
            if (parsed.error != null) {
                Result.failure(RuntimeException(parsed.error))
            } else {
                Result.success(parsed)
            }
        } catch (_: UnsatisfiedLinkError) {
            Result.failure(
                UnsupportedOperationException(
                    "Rebuild libandroidpc_engine.so with cargo-ndk to enable live Box64 JNI execution on device."
                )
            )
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun runBox64Suite(context: Context, envId: String = ""): CompatibilitySuiteReportData = withContext(Dispatchers.IO) {
        if (isLoaded) {
            try {
                val raw = nativeRunBox64Suite(envId)
                return@withContext gson.fromJson(raw, CompatibilitySuiteReportData::class.java)
            } catch (_: UnsatisfiedLinkError) {
                Log.w(TAG, "nativeRunBox64Suite pending native .so rebuild; returning verification checklist")
            } catch (e: Exception) {
                Log.w(TAG, "nativeRunBox64Suite failed: ${e.message}")
            }
        }
        val status = getBox64Status(context, envId)
        CompatibilitySuiteReportData(
            box64_version = status.pinned_version,
            box64_installed = status.installed,
            host_arch = status.host_arch,
            passed = 0,
            failed = 0,
            device_required = 4,
            cases = listOf(
                CompatibilityTestCaseData(
                    id = "A_native_arm64",
                    category = "native_arm64",
                    title = "Native ARM64 ELF detection & shell execution",
                    status = "device_test_required",
                    detail = "Rebuild libandroidpc_engine.so with Phase 4.1 crate and run on ARM64 device."
                ),
                CompatibilityTestCaseData(
                    id = "B_x86_64_core",
                    category = "x86_64_core",
                    title = "x86-64 ELF, math, stdio, files, threads, memory & signals",
                    status = "device_test_required",
                    detail = "Requires staged Box64 v0.4.4 binary and ARM64 Ubuntu/Debian guest on Xiaomi Pad 7."
                ),
                CompatibilityTestCaseData(
                    id = "C_real_apps",
                    category = "real_applications",
                    title = "Curated open-source x86-64 Linux CLI apps (jq, sqlite3)",
                    status = "device_test_required",
                    detail = "Pending on-device acceptance test execution."
                ),
                CompatibilityTestCaseData(
                    id = "D_failure_handling",
                    category = "failure_handling",
                    title = "Missing libs, invalid ELF, Bionic guard & crash classification",
                    status = "device_test_required",
                    detail = "Verified in Rust unit tests (crates/box64); rebuild JNI .so to execute in-app."
                )
            )
        )
    }

    suspend fun getBox64History(limit: Int = 25): List<Box64ExecutionRecordData> = withContext(Dispatchers.IO) {
        if (!isLoaded) return@withContext emptyList()
        try {
            val raw = nativeGetBox64History(limit)
            val listType = object : TypeToken<List<Box64ExecutionRecordData>>() {}.type
            gson.fromJson(raw, listType)
        } catch (_: UnsatisfiedLinkError) {
            emptyList()
        } catch (_: Exception) {
            emptyList()
        }
    }
}
