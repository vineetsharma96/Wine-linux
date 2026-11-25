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
                    var selectedEnvId by remember { mutableStateOf("env-ubuntu") }
                    var wineStatus by remember { mutableStateOf<WineStatusData?>(null) }
                    var wineActionMessage by remember { mutableStateOf<String?>(null) }
                    var isRefreshing by remember { mutableStateOf(false) }

                    var displayResolution by remember { mutableStateOf(1280 to 720) }
                    var activeDisplayPort by remember { mutableIntStateOf(5901) }
                    var audioEnabled by remember { mutableStateOf(true) }
                    var wineArch by remember { mutableStateOf("win64") }
                    var gpuMode by remember { mutableStateOf("software") }

                    val refreshData: () -> Unit = {
                        isRefreshing = true
                        lifecycleScope.launch {
                            NativeBridge.getEngineStatus().onSuccess { engineStatus = it }
                            NativeBridge.runHealthCheck().onSuccess { healthReport = it }
                            NativeBridge.getDiagnostics().onSuccess { diagnostics = it }
                            NativeBridge.listEnvironments().onSuccess { envs ->
                                environments = envs
                                if (envs.isNotEmpty() && envs.none { it.id == selectedEnvId }) {
                                    selectedEnvId = envs.first().id
                                }
                            }
                            NativeBridge.getWineStatus(selectedEnvId).onSuccess { wineStatus = it }
                            isRefreshing = false
                        }
                    }

                    LaunchedEffect(Unit) {
                        NativeBridge.initEngine(applicationContext)
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
                            wineStatus = wineStatus,
                            wineActionMessage = wineActionMessage,
                            isRefreshing = isRefreshing,
                            onRefresh = refreshData,
                            onCreateEnvironment = { currentScreen = Screen.CreateEnvironment },
                            onStartEnvironment = { envId ->
                                selectedEnvId = envId
                                lifecycleScope.launch {
                                    NativeBridge.startEnvironment(envId)
                                        .onSuccess { wineActionMessage = "Started environment '$envId'" }
                                        .onFailure { wineActionMessage = "Start failed: ${it.message}" }
                                    refreshData()
                                }
                            },
                            onStopEnvironment = { envId ->
                                lifecycleScope.launch {
                                    NativeBridge.stopEnvironment(envId)
                                        .onSuccess { wineActionMessage = "Stopped environment '$envId'" }
                                        .onFailure { wineActionMessage = "Stop failed: ${it.message}" }
                                    refreshData()
                                }
                            },
                            onDeleteEnvironment = { envId ->
                                lifecycleScope.launch {
                                    NativeBridge.deleteEnvironment(envId)
                                        .onSuccess { wineActionMessage = "Removed environment '$envId'" }
                                        .onFailure { wineActionMessage = "Remove failed: ${it.message}" }
                                    refreshData()
                                }
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
                                currentScreen = Screen.DisplayViewport
                            },
                            onOpenTerminal = { currentScreen = Screen.Terminal },
                            onOpenVirtualMachines = { currentScreen = Screen.VirtualMachines },
                            onOpenArtifacts = { currentScreen = Screen.Artifacts },
                            onOpenSettings = { currentScreen = Screen.Settings }
                        )
                        Screen.Terminal -> TerminalScreen(
                            envId = selectedEnvId,
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                        Screen.DisplayViewport -> DisplayViewportScreen(
                            displayPort = activeDisplayPort,
                            targetWidth = displayResolution.first,
                            targetHeight = displayResolution.second,
                            audioEnabled = audioEnabled,
                            onDisconnect = { currentScreen = Screen.Dashboard }
                        )
                        Screen.VirtualMachines -> VirtualMachinesScreen(
                            onOpenVmDisplay = { vncPort ->
                                activeDisplayPort = vncPort
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
                            onCreated = { distro ->
                                selectedEnvId = "env-$distro"
                                refreshData()
                                currentScreen = Screen.DisplayViewport
                            }
                        )
                        Screen.Settings -> SettingsScreen(
                            displayResolution = displayResolution,
                            audioEnabled = audioEnabled,
                            wineArch = wineArch,
                            gpuMode = gpuMode,
                            onResolutionChange = { displayResolution = it },
                            onAudioEnabledChange = { audioEnabled = it },
                            onWineArchChange = { wineArch = it },
                            onGpuModeChange = { newMode ->
                                gpuMode = newMode
                                lifecycleScope.launch {
                                    NativeBridge.setGpuAccelerationMode(newMode)
                                }
                            },
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                    }
                }
            }
        }
    }
}
