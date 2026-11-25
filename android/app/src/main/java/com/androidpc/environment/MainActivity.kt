package com.androidpc.environment

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.lifecycle.lifecycleScope
import com.androidpc.environment.bridge.EngineStatusData
import com.androidpc.environment.bridge.EnvironmentConfigData
import com.androidpc.environment.bridge.HealthReportData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.bridge.SystemDiagnosticsData
import com.androidpc.environment.bridge.WineStatusData
import com.androidpc.environment.ui.screens.*
import com.androidpc.environment.ui.theme.AndroidPCEnvironmentTheme
import com.androidpc.environment.ui.theme.BgDark
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    enum class Screen {
        Splash,
        Onboarding,
        Dashboard,
        Terminal,
        HealthCheck,
        SystemInfo,
        CreateEnvironment,
        EnvironmentDetails,
        EnvironmentBackups,
        DisplayViewport,
        VirtualMachines,
        Artifacts,
        Settings
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            AndroidPCEnvironmentTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = BgDark
                ) {
                    var currentScreen by remember { mutableStateOf(Screen.Splash) }
                    var engineStatus by remember { mutableStateOf<EngineStatusData?>(null) }
                    var healthReport by remember { mutableStateOf<HealthReportData?>(null) }
                    var diagnostics by remember { mutableStateOf<SystemDiagnosticsData?>(null) }
                    var environments by remember { mutableStateOf<List<EnvironmentConfigData>>(emptyList()) }
                    var backupSourceId by remember { mutableStateOf<String?>(null) }
                    var environmentStates by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
                    var environmentStorage by remember { mutableStateOf<Map<String, Long?>>(emptyMap()) }
                    var environmentActionMessage by remember { mutableStateOf<String?>(null) }
                    val prefs = remember { applicationContext.getSharedPreferences("androidpc_prefs", MODE_PRIVATE) }
                    var defaultEnvId by remember { mutableStateOf(prefs.getString("default_env_id", "") ?: "") }
                    var defaultShell by remember { mutableStateOf(prefs.getString("default_shell", "/bin/bash") ?: "/bin/bash") }
                    var selectedEnvId by remember { mutableStateOf(defaultEnvId.ifEmpty { "env-ubuntu" }) }
                    var wineStatus by remember { mutableStateOf<WineStatusData?>(null) }
                    var wineActionMessage by remember { mutableStateOf<String?>(null) }
                    var isRefreshing by remember { mutableStateOf(false) }

                    var displayResolution by remember {
                        mutableStateOf(
                            prefs.getInt("display_width", 1280) to prefs.getInt("display_height", 720)
                        )
                    }
                    var activeDisplayPort by remember { mutableIntStateOf(5901) }
                    var managedDesktop by remember { mutableStateOf(true) }
                    var audioEnabled by remember { mutableStateOf(prefs.getBoolean("audio_enabled", true)) }
                    var wineArch by remember { mutableStateOf(prefs.getString("wine_arch", "win64") ?: "win64") }
                    var gpuMode by remember { mutableStateOf(prefs.getString("gpu_mode", "software") ?: "software") }

                    val refreshData: () -> Unit = {
                        isRefreshing = true
                        lifecycleScope.launch {
                            NativeBridge.getEngineStatus().onSuccess { engineStatus = it }
                            NativeBridge.runHealthCheck().onSuccess { healthReport = it }
                            NativeBridge.getDiagnostics().onSuccess { diagnostics = it }
                            NativeBridge.listEnvironments().onSuccess { envs ->
                                environments = envs
                                environmentStates = envs.associate { it.id to NativeBridge.getEnvironmentState(it.id) }
                                environmentStorage = envs.associate { it.id to NativeBridge.environmentStorageBytes(it.id) }
                                if (envs.isNotEmpty() && envs.none { it.id == selectedEnvId }) {
                                    selectedEnvId = envs.firstOrNull { it.id == defaultEnvId }?.id ?: envs.first().id
                                }
                                if (envs.isNotEmpty() && defaultEnvId.isEmpty()) {
                                    defaultEnvId = envs.first().id
                                    prefs.edit().putString("default_env_id", defaultEnvId).apply()
                                }
                            }
                            NativeBridge.getWineStatus(selectedEnvId).onSuccess { wineStatus = it }
                            isRefreshing = false
                        }
                    }

                    LaunchedEffect(Unit) {
                        NativeBridge.initEngine(applicationContext)
                        NativeBridge.setGpuAccelerationMode(gpuMode)
                        refreshData()
                    }

                    when (currentScreen) {
                        Screen.Splash -> SplashScreen {
                            currentScreen = Screen.Dashboard
                        }
                        Screen.Onboarding -> OnboardingScreen {
                            currentScreen = Screen.Dashboard
                        }
                        Screen.Dashboard -> DashboardScreen(
                            engineStatus = engineStatus,
                            healthReport = healthReport,
                            environments = environments,
                            environmentStates = environmentStates,
                            environmentStorage = environmentStorage,
                            availableStorageBytes = diagnostics?.available_storage_bytes,
                            environmentActionMessage = environmentActionMessage,
                            wineStatus = wineStatus,
                            wineActionMessage = wineActionMessage,
                            isRefreshing = isRefreshing,
                            onRefresh = refreshData,
                            onCreateEnvironment = { currentScreen = Screen.CreateEnvironment },
                            onStartEnvironment = { envId ->
                                selectedEnvId = envId
                                lifecycleScope.launch {
                                    NativeBridge.startEnvironment(envId)
                                        .onSuccess { environmentActionMessage = "Environment started" }
                                        .onFailure { environmentActionMessage = "Start failed: ${it.message}" }
                                    refreshData()
                                }
                            },
                            onStopEnvironment = { envId ->
                                lifecycleScope.launch {
                                    NativeBridge.stopEnvironment(envId)
                                        .onSuccess { environmentActionMessage = "Environment stopped" }
                                        .onFailure { environmentActionMessage = "Stop failed: ${it.message}" }
                                    refreshData()
                                }
                            },
                            onDeleteEnvironment = { envId ->
                                lifecycleScope.launch {
                                    NativeBridge.deleteEnvironment(envId)
                                        .onSuccess { environmentActionMessage = "Environment removed" }
                                        .onFailure { environmentActionMessage = "Remove failed: ${it.message}" }
                                    refreshData()
                                }
                            },
                            onRecoverEnvironment = { envId, restart ->
                                lifecycleScope.launch {
                                    NativeBridge.recoverEnvironment(envId, restart)
                                        .onSuccess { environmentActionMessage = if (restart) "Environment restarted" else "Guest configuration repaired" }
                                        .onFailure { environmentActionMessage = it.message }
                                    refreshData()
                                }
                            },
                            onOpenEnvironmentTerminal = { envId ->
                                selectedEnvId = envId
                                currentScreen = Screen.Terminal
                            },
                            onOpenEnvironmentDetails = { envId ->
                                selectedEnvId = envId
                                currentScreen = Screen.EnvironmentDetails
                            },
                            onOpenBackups = {
                                backupSourceId = null
                                currentScreen = Screen.EnvironmentBackups
                            },
                            onInitWinePrefix = {
                                lifecycleScope.launch {
                                    NativeBridge.initWinePrefix(selectedEnvId, "/home/user/.wine", wineArch)
                                        .onSuccess {
                                            wineActionMessage = "Wine C: drive prefix ($wineArch) initialized for $selectedEnvId"
                                        }
                                        .onFailure {
                                            wineActionMessage = "Wine init error: ${it.message}"
                                        }
                                    refreshData()
                                }
                            },
                            onLaunchWineApp = { exePath, args ->
                                lifecycleScope.launch {
                                    NativeBridge.launchWineApp(selectedEnvId, exePath, args)
                                        .onSuccess { cmd ->
                                            val pidNote = cmd.spawned_pid?.let { " (PID $it)" } ?: " (command staged)"
                                            wineActionMessage = "${cmd.program} ${cmd.args.joinToString(" ")}$pidNote"
                                        }
                                        .onFailure {
                                            wineActionMessage = "Launch error: ${it.message}"
                                        }
                                    refreshData()
                                }
                            },
                            onKillWineSession = {
                                lifecycleScope.launch {
                                    NativeBridge.killWineSession(selectedEnvId)
                                        .onSuccess { wineActionMessage = "Terminated wineserver for $selectedEnvId" }
                                        .onFailure { wineActionMessage = "Kill error: ${it.message}" }
                                    refreshData()
                                }
                            },
                            onOpenHealthCheck = { currentScreen = Screen.HealthCheck },
                            onOpenSystemInfo = { currentScreen = Screen.SystemInfo },
                            onOpenDisplayViewport = {
                                activeDisplayPort = 5901
                                managedDesktop = true
                                currentScreen = Screen.DisplayViewport
                            },
                            onOpenTerminal = { currentScreen = Screen.Terminal },
                            onOpenVirtualMachines = { currentScreen = Screen.VirtualMachines },
                            onOpenArtifacts = { currentScreen = Screen.Artifacts },
                            onOpenSettings = { currentScreen = Screen.Settings }
                        )
                        Screen.Terminal -> TerminalScreen(
                            envId = selectedEnvId,
                            defaultShell = defaultShell,
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                        Screen.EnvironmentDetails -> environments.firstOrNull { it.id == selectedEnvId }?.let { env ->
                            EnvironmentDetailsScreen(env, environmentStates[env.id], environmentStorage[env.id], onBackups = {
                                backupSourceId = env.id
                                currentScreen = Screen.EnvironmentBackups
                            }) {
                                refreshData()
                                currentScreen = Screen.Dashboard
                            }
                        } ?: Unit
                        Screen.EnvironmentBackups -> EnvironmentBackupsScreen(
                            source = environments.firstOrNull { it.id == backupSourceId },
                            onBack = {
                                currentScreen = if (backupSourceId == null) Screen.Dashboard else Screen.EnvironmentDetails
                            },
                            onRestored = { id ->
                                selectedEnvId = id
                                refreshData()
                                currentScreen = Screen.Dashboard
                            },
                        )
                        Screen.DisplayViewport -> DisplayViewportScreen(
                            environmentId = environments.firstOrNull { it.id == defaultEnvId }?.id.orEmpty(),
                            displayPort = activeDisplayPort,
                            managedDesktop = managedDesktop,
                            targetWidth = displayResolution.first,
                            targetHeight = displayResolution.second,
                            audioEnabled = audioEnabled,
                            onDisconnect = {
                                currentScreen = Screen.Dashboard
                                refreshData()
                            }
                        )
                        Screen.VirtualMachines -> VirtualMachinesScreen(
                            onOpenVmDisplay = { vncPort ->
                                activeDisplayPort = vncPort
                                managedDesktop = false
                                currentScreen = Screen.DisplayViewport
                            },
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                        Screen.Artifacts -> ArtifactsScreen(
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                        Screen.HealthCheck -> HealthStatusScreen(
                            healthReport = healthReport,
                            onBack = { currentScreen = Screen.Dashboard },
                            onRunCheck = refreshData
                        )
                        Screen.SystemInfo -> SystemInfoScreen(
                            diagnostics = diagnostics,
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                        Screen.CreateEnvironment -> CreateEnvironmentScreen(
                            onBack = { currentScreen = Screen.Dashboard },
                            onCreated = { id ->
                                selectedEnvId = id
                                refreshData()
                                currentScreen = Screen.Terminal
                            }
                        )
                        Screen.Settings -> SettingsScreen(
                            displayResolution = displayResolution,
                            audioEnabled = audioEnabled,
                            wineArch = wineArch,
                            gpuMode = gpuMode,
                            environments = environments,
                            defaultEnvId = defaultEnvId,
                            defaultShell = defaultShell,
                            onResolutionChange = { res ->
                                displayResolution = res
                                prefs.edit().putInt("display_width", res.first).putInt("display_height", res.second).apply()
                            },
                            onAudioEnabledChange = { enabled ->
                                audioEnabled = enabled
                                prefs.edit().putBoolean("audio_enabled", enabled).apply()
                            },
                            onWineArchChange = { arch ->
                                wineArch = arch
                                prefs.edit().putString("wine_arch", arch).apply()
                            },
                            onGpuModeChange = { newMode ->
                                gpuMode = newMode
                                prefs.edit().putString("gpu_mode", newMode).apply()
                                lifecycleScope.launch {
                                    NativeBridge.setGpuAccelerationMode(newMode)
                                }
                            },
                            onDefaultEnvChange = { id ->
                                defaultEnvId = id
                                selectedEnvId = id
                                prefs.edit().putString("default_env_id", id).apply()
                            },
                            onDefaultShellChange = { sh ->
                                defaultShell = sh
                                prefs.edit().putString("default_shell", sh).apply()
                            },
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                    }
                }
            }
        }
    }
}
