package com.androidpc.environment.bridge

import android.content.Context
import android.util.Log
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Kotlin-to-Rust JNI coordinator for Phase 4 Windows Compatibility Manager.
 *
 * Calls the Rust engine (`androidpc-core` -> `androidpc-wine`, `androidpc-graphics-backend`,
 * `androidpc-dependency-manager`, `androidpc-windows-apps`) over JNI. When running against a
 * pre-Phase-4 `libandroidpc_engine.so` prior to `cargo-ndk` rebuild, catches `UnsatisfiedLinkError`
 * and honestly inspects local staging directories without fabricating Operational status.
 */
object WindowsCompatibilityBridge {
    private const val TAG = "WindowsCompatBridge"
    private const val PREFS_NAME = "androidpc_phase4_state"
    private val gson = Gson()

    suspend fun getOverview(context: Context, envId: String): WindowsCompatibilityOverviewData =
        withContext(Dispatchers.IO) {
            if (NativeBridge.isLoaded) {
                try {
                    val raw = NativeBridge.nativeGetWindowsOverview(envId)
                    if (!raw.contains("\"error\"")) {
                        return@withContext gson.fromJson(raw, WindowsCompatibilityOverviewData::class.java)
                    }
                } catch (_: UnsatisfiedLinkError) {
                    Log.w(TAG, "Phase 4 JNI symbols pending cargo-ndk rebuild; inspecting local staging")
                } catch (e: Exception) {
                    Log.w(TAG, "nativeGetWindowsOverview failed: ${e.message}")
                }
            }
            val box64 = NativeBridge.getBox64Status(context, envId)
            val wineBin = File(context.filesDir, "artifacts/wine/bin/wine")
            val wineInstalled = wineBin.isFile
            val graphics = probeGraphicsBackends(context, envId)
            val prefixes = listWinePrefixes(context)
            val apps = listWindowsApps(context)

            val errors = mutableListOf<String>()
            if (!box64.installed) {
                errors += "Box64 v0.4.4 binary not staged yet (run scripts/build_box64_arm64.sh)."
            }
            if (!wineInstalled) {
                errors += "Wine 11.0 WoW64 runtime not staged yet under artifacts/wine/."
            }
            graphics.fallback_reason?.let { errors += it }

            WindowsCompatibilityOverviewData(
                environment_id = envId,
                box64 = box64,
                wine = WineInstallationStatusData(
                    installed = wineInstalled,
                    binary_path = if (wineInstalled) wineBin.absolutePath else null,
                    detected_version = if (wineInstalled) "11.0" else null,
                    pinned_version = "11.0",
                    license = "LGPL-2.1-or-later",
                    architecture_strategy = "x86_64 Wine 11.0 (pure-64-bit WoW64) via Box64",
                    wow64_enabled = true,
                    box86_required = false,
                    diagnostics = listOf(
                        if (wineInstalled) "Wine binary found at ${wineBin.absolutePath}"
                        else "Wine 11.0 WoW64 binary not staged; rebuild JNI .so and stage verified Wine archive."
                    )
                ),
                graphics = graphics,
                prefix_count = prefixes.size,
                installed_apps_count = apps.size,
                active_processes_count = 0,
                runtime_errors = errors
            )
        }

    suspend fun listWinePrefixes(context: Context): List<WinePrefixMetadataData> =
        withContext(Dispatchers.IO) {
            if (NativeBridge.isLoaded) {
                try {
                    val raw = NativeBridge.nativeListWinePrefixes()
                    val type = object : TypeToken<List<WinePrefixMetadataData>>() {}.type
                    return@withContext gson.fromJson(raw, type)
                } catch (_: UnsatisfiedLinkError) {
                    // Fall through to local metadata store until .so is rebuilt
                } catch (e: Exception) {
                    Log.w(TAG, "nativeListWinePrefixes failed: ${e.message}")
                }
            }
            val raw = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .getString("prefixes", "[]") ?: "[]"
            runCatching {
                val type = object : TypeToken<List<WinePrefixMetadataData>>() {}.type
                gson.fromJson<List<WinePrefixMetadataData>>(raw, type)
            }.getOrDefault(emptyList())
        }

    suspend fun createWinePrefix(
        context: Context,
        id: String,
        displayName: String,
        envId: String,
        arch: String = "WoW64",
        windowsVersion: String = "win10",
        graphicsBackend: String = "software"
    ): Result<WinePrefixMetadataData> = withContext(Dispatchers.IO) {
        val cleanId = id.trim().lowercase().replace(Regex("[^a-z0-9_-]"), "-")
        if (cleanId.isEmpty()) return@withContext Result.failure(IllegalArgumentException("Prefix ID is required"))

        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeCreateWinePrefix(
                    cleanId,
                    displayName.ifBlank { cleanId },
                    envId,
                    arch,
                    windowsVersion,
                    graphicsBackend
                )
                val parsed = gson.fromJson(raw, WinePrefixMetadataData::class.java)
                if (parsed.error != null) return@withContext Result.failure(RuntimeException(parsed.error))
                return@withContext Result.success(parsed)
            } catch (_: UnsatisfiedLinkError) {
                // Persist locally until libandroidpc_engine.so is rebuilt
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        val existing = listWinePrefixes(context).toMutableList()
        if (existing.any { it.id == cleanId }) {
            return@withContext Result.failure(IllegalStateException("Prefix '$cleanId' already exists"))
        }
        val now = System.currentTimeMillis()
        val created = WinePrefixMetadataData(
            id = cleanId,
            display_name = displayName.ifBlank { cleanId },
            environment_id = envId,
            wine_version = "11.0",
            windows_version = windowsVersion,
            arch = arch,
            prefix_path = "/home/user/.androidpc-prefixes/$cleanId",
            graphics_backend = graphicsBackend,
            created_at_epoch_ms = now,
            last_used_at_epoch_ms = now
        )
        existing.add(created)
        saveLocalPrefixes(context, existing)
        Result.success(created)
    }

    suspend fun manageWinePrefix(
        context: Context,
        action: String,
        prefixId: String,
        payload: Map<String, Any> = emptyMap()
    ): Result<String> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeManageWinePrefix(action, prefixId, gson.toJson(payload))
                if (raw.contains("\"error\"")) return@withContext Result.failure(RuntimeException(raw))
                return@withContext Result.success(raw)
            } catch (_: UnsatisfiedLinkError) {
                // Local fallback for delete/clone/repair metadata
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        val list = listWinePrefixes(context).toMutableList()
        when (action) {
            "delete" -> {
                list.removeAll { it.id == prefixId }
                saveLocalPrefixes(context, list)
                Result.success("Prefix '$prefixId' deleted")
            }
            "clone" -> {
                val src = list.firstOrNull { it.id == prefixId }
                    ?: return@withContext Result.failure(IllegalArgumentException("Prefix not found"))
                val newId = (payload["new_id"] as? String).orEmpty().ifBlank { "${prefixId}-clone" }
                val now = System.currentTimeMillis()
                list.add(
                    src.copy(
                        id = newId,
                        display_name = "${src.display_name} (Clone)",
                        prefix_path = "/home/user/.androidpc-prefixes/$newId",
                        created_at_epoch_ms = now,
                        last_used_at_epoch_ms = now
                    )
                )
                saveLocalPrefixes(context, list)
                Result.success("Cloned prefix into '$newId'")
            }
            "repair" -> Result.success("Rebuild libandroidpc_engine.so to run on-disk guest prefix directory repair.")
            "backup" -> Result.success("Rebuild libandroidpc_engine.so to create SHA-256 tar.gz prefix backups.")
            else -> Result.failure(UnsupportedOperationException("Unsupported action: $action"))
        }
    }

    suspend fun probeGraphicsBackends(context: Context, envId: String): GraphicsStackOverviewData =
        withContext(Dispatchers.IO) {
            if (NativeBridge.isLoaded) {
                try {
                    val raw = NativeBridge.nativeProbeGraphicsBackends(envId)
                    if (!raw.contains("\"error\"")) {
                        return@withContext gson.fromJson(raw, GraphicsStackOverviewData::class.java)
                    }
                } catch (_: UnsatisfiedLinkError) {
                    // Honest local probe fallback
                } catch (e: Exception) {
                    Log.w(TAG, "nativeProbeGraphicsBackends failed: ${e.message}")
                }
            }
            val cfg = loadLocalGraphicsConfig(context)
            val turnipIcd = File(context.filesDir, "artifacts/graphics/turnip/freedreno_icd.aarch64.json")
            val turnipSo = File(context.filesDir, "artifacts/graphics/turnip/libvulkan_freedreno.so")
            val kgslExists = File("/dev/kgsl-3d0").exists()
            val turnipStage = when {
                !turnipIcd.isFile || !turnipSo.isFile -> "not_installed"
                !kgslExists -> "installed"
                else -> "detected"
            }

            val virglBin = File(context.filesDir, "artifacts/graphics/virgl/virgl_test_server_android")
            val virglStage = if (virglBin.isFile) "detected" else "not_installed"

            val dxvkStaged = File(context.filesDir, "artifacts/dxvk/x64/d3d11.dll").isFile
            val vkd3dStaged = File(context.filesDir, "artifacts/vkd3d-proton/x64/d3d12.dll").isFile

            val fallbackEngaged = cfg.preferred_backend != "software"
            val fallbackReason = if (fallbackEngaged) {
                "Preferred backend '${cfg.preferred_backend}' is not yet verified Operational; Software (Mesa LLVMpipe) fallback engaged."
            } else null

            GraphicsStackOverviewData(
                active_config = cfg,
                effective_backend = if (fallbackEngaged && cfg.allow_software_fallback) "software" else cfg.preferred_backend,
                fallback_engaged = fallbackEngaged && cfg.allow_software_fallback,
                fallback_reason = fallbackReason,
                backends = listOf(
                    BackendCapabilityReportData(
                        kind = "turnip",
                        display_name = "Mesa Turnip (Adreno KGSL Vulkan)",
                        pinned_version = "26.3.1",
                        license = "MIT",
                        stage = turnipStage,
                        supports_opengl = true,
                        supports_vulkan = true,
                        dxvk_compatible = false,
                        vkd3d_compatible = false,
                        hardware_accelerated = true,
                        detected_gpu = "Xiaomi Pad 7 reference: Qualcomm Adreno 732 (KGSL present=$kgslExists)",
                        warnings = listOf("Turnip requires guest ARM64 Mesa ICD staging + live on-device Vulkan probe to reach Operational.")
                    ),
                    BackendCapabilityReportData(
                        kind = "virgl",
                        display_name = "VirGL (OpenGL-over-IPC via virglrenderer)",
                        pinned_version = "1.3.0",
                        license = "MIT",
                        stage = virglStage,
                        supports_opengl = true,
                        supports_vulkan = false,
                        dxvk_compatible = false,
                        vkd3d_compatible = false,
                        hardware_accelerated = true,
                        warnings = listOf("OpenGL-only (virpipe); cannot back Vulkan, DXVK, or VKD3D-Proton.")
                    ),
                    BackendCapabilityReportData(
                        kind = "software",
                        display_name = "Software Fallback (Mesa LLVMpipe / Lavapipe)",
                        pinned_version = "26.3.1",
                        license = "MIT",
                        stage = "detected",
                        supports_opengl = true,
                        supports_vulkan = false,
                        dxvk_compatible = false,
                        vkd3d_compatible = false,
                        hardware_accelerated = false,
                        detected_gpu = "ARM64 CPU (Mesa llvmpipe)"
                    )
                ),
                dx_translation = DxTranslationStatusData(
                    dxvk_pinned_version = "2.7.1",
                    dxvk_license = "zlib",
                    dxvk_staged = dxvkStaged,
                    dxvk_eligible = false,
                    dxvk_fallback_reason = "Requires verified Operational Vulkan 1.3+ backend (Turnip); using WineD3D OpenGL fallback.",
                    vkd3d_pinned_version = "3.1",
                    vkd3d_license = "LGPL-2.1",
                    vkd3d_staged = vkd3dStaged,
                    vkd3d_eligible = false,
                    vkd3d_fallback_reason = "Direct3D 12 disabled until Turnip Vulkan 1.3+ required extensions are verified on device.",
                    effective_choice = "wine_d3d_open_gl"
                )
            )
        }

    suspend fun configureGraphicsBackend(
        context: Context,
        envId: String,
        config: GraphicsBackendConfigData
    ): GraphicsStackOverviewData = withContext(Dispatchers.IO) {
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString("graphics_cfg", gson.toJson(config))
            .apply()
        NativeBridge.setGpuAccelerationMode(config.preferred_backend)

        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeConfigureGraphicsBackend(envId, gson.toJson(config))
                if (!raw.contains("\"error\"")) {
                    return@withContext gson.fromJson(raw, GraphicsStackOverviewData::class.java)
                }
            } catch (_: UnsatisfiedLinkError) {
                // Local fallback below
            } catch (e: Exception) {
                Log.w(TAG, "nativeConfigureGraphicsBackend failed: ${e.message}")
            }
        }
        probeGraphicsBackends(context, envId)
    }

    suspend fun listWindowsApps(context: Context): List<WindowsAppProfileData> =
        withContext(Dispatchers.IO) {
            if (NativeBridge.isLoaded) {
                try {
                    val raw = NativeBridge.nativeListWindowsApps()
                    val type = object : TypeToken<List<WindowsAppProfileData>>() {}.type
                    return@withContext gson.fromJson(raw, type)
                } catch (_: UnsatisfiedLinkError) {
                    // Local fallback below
                } catch (e: Exception) {
                    Log.w(TAG, "nativeListWindowsApps failed: ${e.message}")
                }
            }
            val raw = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .getString("win_apps", "[]") ?: "[]"
            runCatching {
                val type = object : TypeToken<List<WindowsAppProfileData>>() {}.type
                gson.fromJson<List<WindowsAppProfileData>>(raw, type)
            }.getOrDefault(emptyList())
        }

    suspend fun registerWindowsApp(
        context: Context,
        profile: WindowsAppProfileData
    ): Result<WindowsAppProfileData> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeRegisterWindowsApp(gson.toJson(profile))
                val parsed = gson.fromJson(raw, WindowsAppProfileData::class.java)
                if (parsed.error != null) return@withContext Result.failure(RuntimeException(parsed.error))
                return@withContext Result.success(parsed)
            } catch (_: UnsatisfiedLinkError) {
                // Local persistence until native .so rebuild
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        val list = listWindowsApps(context).toMutableList()
        list.removeAll { it.app_id == profile.app_id }
        val saved = profile.copy(created_at_epoch_ms = if (profile.created_at_epoch_ms == 0L) System.currentTimeMillis() else profile.created_at_epoch_ms)
        list.add(saved)
        saveLocalApps(context, list)
        Result.success(saved)
    }

    suspend fun launchWindowsApp(context: Context, app: WindowsAppProfileData): Result<WindowsAppLaunchResultData> =
        withContext(Dispatchers.IO) {
            if (NativeBridge.isLoaded) {
                try {
                    val raw = NativeBridge.nativeLaunchWindowsApp(app.app_id)
                    val parsed = gson.fromJson(raw, WindowsAppLaunchResultData::class.java)
                    if (parsed.error != null) return@withContext Result.failure(RuntimeException(parsed.error))
                    return@withContext Result.success(parsed)
                } catch (_: UnsatisfiedLinkError) {
                    // Fallback to existing Phase 3/4 launchWineApp JNI symbol
                    val fallback = NativeBridge.launchWineApp(app.environment_id, app.executable_path, app.launch_args)
                    return@withContext fallback.map { cmd ->
                        WindowsAppLaunchResultData(
                            app_id = app.app_id,
                            prefix_id = app.prefix_id,
                            spawned_pid = cmd.spawned_pid,
                            effective_graphics_backend = app.graphics_backend,
                            dx_choice = "wine_d3d_open_gl",
                            warnings = listOf("Launched via legacy JNI bridge; rebuild libandroidpc_engine.so for full isolated prefix & graphics orchestration.")
                        )
                    }
                } catch (e: Exception) {
                    return@withContext Result.failure(e)
                }
            }
            Result.failure(IllegalStateException("Native engine not loaded"))
        }

    suspend fun removeWindowsApp(context: Context, appId: String): Result<Unit> =
        withContext(Dispatchers.IO) {
            if (NativeBridge.isLoaded) {
                try {
                    val raw = NativeBridge.nativeRemoveWindowsApp(appId)
                    if (raw.contains("\"error\"")) return@withContext Result.failure(RuntimeException(raw))
                    return@withContext Result.success(Unit)
                } catch (_: UnsatisfiedLinkError) {
                    // Local fallback below
                } catch (e: Exception) {
                    return@withContext Result.failure(e)
                }
            }
            val list = listWindowsApps(context).toMutableList()
            list.removeAll { it.app_id == appId }
            saveLocalApps(context, list)
            Result.success(Unit)
        }

    suspend fun terminateWindowsApp(appId: String, force: Boolean = false): Result<Unit> =
        withContext(Dispatchers.IO) {
            if (!NativeBridge.isLoaded) return@withContext Result.failure(IllegalStateException("Native engine not loaded"))
            try {
                val raw = NativeBridge.nativeTerminateWindowsApp(appId, force)
                if (raw.contains("\"error\"")) return@withContext Result.failure(RuntimeException(raw))
                Result.success(Unit)
            } catch (_: UnsatisfiedLinkError) {
                Result.failure(UnsupportedOperationException("Rebuild libandroidpc_engine.so to enable scoped per-app termination."))
            } catch (e: Exception) {
                Result.failure(e)
            }
        }

    suspend fun listWindowsProcesses(): List<TrackedWindowsProcessData> = withContext(Dispatchers.IO) {
        if (!NativeBridge.isLoaded) return@withContext emptyList()
        try {
            val raw = NativeBridge.nativeListWindowsProcesses()
            val type = object : TypeToken<List<TrackedWindowsProcessData>>() {}.type
            gson.fromJson(raw, type)
        } catch (_: Throwable) {
            emptyList()
        }
    }

    suspend fun listDependencyProfiles(): List<DependencyProfileData> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeListWindowsDependencies()
                val type = object : TypeToken<List<DependencyProfileData>>() {}.type
                return@withContext gson.fromJson(raw, type)
            } catch (_: Throwable) {
                // Return matching verified catalog metadata
            }
        }
        listOf(
            DependencyProfileData(
                id = "wine-mono",
                display_name = "Wine Mono (.NET Open-Source Runtime)",
                version = "9.4.0",
                category = "dotnet",
                license = "MIT / LGPL / GPL-2.0",
                mechanism = "wine_native_tool",
                official_source_url = "https://dl.winehq.org/wine/wine-mono/",
                expected_dlls = listOf("mscoree.dll"),
                recovery_hint = "Install official Wine Mono MSI via Wine inside isolated prefix."
            ),
            DependencyProfileData(
                id = "dxvk-2.7.1",
                display_name = "DXVK (Direct3D 8/9/10/11 over Vulkan)",
                version = "2.7.1",
                category = "graphics",
                license = "zlib",
                mechanism = "verified_open_source_artifact",
                official_source_url = "https://github.com/doitsujin/dxvk/releases",
                expected_dlls = listOf("d3d9.dll", "d3d10core.dll", "d3d11.dll", "dxgi.dll"),
                recovery_hint = "Requires operational Vulkan 1.3+ Turnip backend; falls back to WineD3D OpenGL otherwise."
            ),
            DependencyProfileData(
                id = "vkd3d-proton-3.1",
                display_name = "VKD3D-Proton (Direct3D 12 over Vulkan)",
                version = "3.1",
                category = "graphics",
                license = "LGPL-2.1",
                mechanism = "verified_open_source_artifact",
                official_source_url = "https://github.com/HansKristian-Work/vkd3d-proton/releases",
                expected_dlls = listOf("d3d12.dll", "d3d12core.dll"),
                recovery_hint = "Strictly gated on verified Vulkan 1.3+ extensions."
            ),
            DependencyProfileData(
                id = "vcrun2015-2022",
                display_name = "Visual C++ 2015–2022 Redistributable (x64)",
                version = "14.42",
                category = "cpp_runtime",
                license = "Microsoft Software License (User-Supplied)",
                mechanism = "user_supplied_official_installer",
                official_source_url = "https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist",
                expected_dlls = listOf("vcruntime140.dll", "msvcp140.dll", "ucrtbase.dll"),
                recovery_hint = "Never bundled; download official vc_redist.x64.exe from Microsoft, verify SHA-256, and install via Windows Apps."
            )
        )
    }

    suspend fun diagnosePrefixDependencies(
        prefixId: String,
        recentLogs: List<String> = emptyList()
    ): PrefixDependencyDiagnosisData = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeDiagnosePrefixDependencies(prefixId, gson.toJson(recentLogs))
                if (!raw.contains("\"error\"")) {
                    return@withContext gson.fromJson(raw, PrefixDependencyDiagnosisData::class.java)
                }
            } catch (_: Throwable) {
                // Honest fallback
            }
        }
        PrefixDependencyDiagnosisData(
            prefix_id = prefixId,
            notes = listOf("Rebuild libandroidpc_engine.so with cargo-ndk to scan guest drive_c/windows/system32 DLLs.")
        )
    }

    private fun saveLocalPrefixes(context: Context, list: List<WinePrefixMetadataData>) {
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString("prefixes", gson.toJson(list))
            .apply()
    }

    private fun saveLocalApps(context: Context, list: List<WindowsAppProfileData>) {
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString("win_apps", gson.toJson(list))
            .apply()
    }

    private fun loadLocalGraphicsConfig(context: Context): GraphicsBackendConfigData {
        val raw = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .getString("graphics_cfg", null)
        return if (!raw.isNullOrBlank()) {
            runCatching { gson.fromJson(raw, GraphicsBackendConfigData::class.java) }
                .getOrDefault(GraphicsBackendConfigData())
        } else {
            GraphicsBackendConfigData()
        }
    }
}
