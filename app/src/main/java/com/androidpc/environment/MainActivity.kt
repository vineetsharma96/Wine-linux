package com.androidpc.environment

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.lifecycleScope
import com.androidpc.environment.bridge.*
import com.androidpc.environment.ui.screens.*
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    enum class Screen {
        Splash,
        Onboarding,
        Dashboard,
        WindowsCompatibility,
        Compatibility,
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
        enableEdgeToEdge()

        setContent {
            AndroidPCEnvironmentTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = BgDark
                ) {
                    val drawerState = rememberDrawerState(initialValue = DrawerValue.Closed)
                    val uiScope = rememberCoroutineScope()

                    var currentScreen by remember { mutableStateOf(Screen.Splash) }
                    var engineStatus by remember { mutableStateOf<EngineStatusData?>(null) }
                    var healthReport by remember { mutableStateOf<HealthReportData?>(null) }
                    var diagnostics by remember { mutableStateOf<SystemDiagnosticsData?>(null) }
                    var box64Status by remember { mutableStateOf<Box64InstallationStatusData?>(null) }
                    var windowsOverview by remember { mutableStateOf<WindowsCompatibilityOverviewData?>(null) }
                    var windowsCompatInitialTab by remember { mutableIntStateOf(0) }
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
                            box64Status = NativeBridge.getBox64Status(applicationContext, selectedEnvId)
                            windowsOverview = WindowsCompatibilityBridge.getOverview(applicationContext, selectedEnvId)
                            NativeBridge.getWineStatus(selectedEnvId).onSuccess { wineStatus = it }
                            isRefreshing = false
                        }
                    }

                    LaunchedEffect(Unit) {
                        NativeBridge.initEngine(applicationContext)
                        NativeBridge.setGpuAccelerationMode(gpuMode)
                        refreshData()
                    }

                    val navigateFromDrawer: (Screen) -> Unit = { target ->
                        uiScope.launch { drawerState.close() }
                        if (target == Screen.DisplayViewport) {
                            activeDisplayPort = 5901
                            managedDesktop = true
                        } else if (target == Screen.EnvironmentBackups) {
                            backupSourceId = null
                        }
                        currentScreen = target
                    }

                    BackHandler(enabled = drawerState.isOpen) {
                        uiScope.launch { drawerState.close() }
                    }

                    BackHandler(enabled = !drawerState.isOpen && currentScreen !in setOf(Screen.Splash, Screen.Dashboard, Screen.Onboarding)) {
                        currentScreen = if (currentScreen == Screen.EnvironmentBackups && backupSourceId != null) {
                            Screen.EnvironmentDetails
                        } else {
                            Screen.Dashboard
                        }
                    }

                    ModalNavigationDrawer(
                        drawerState = drawerState,
                        gesturesEnabled = currentScreen == Screen.Dashboard,
                        drawerContent = {
                            StarNavigationDrawerSheet(
                                currentScreen = currentScreen,
                                windowsCompatTab = windowsCompatInitialTab,
                                engineStatus = engineStatus,
                                onSelect = navigateFromDrawer,
                                onSelectWindowsTab = { tab ->
                                    uiScope.launch { drawerState.close() }
                                    windowsCompatInitialTab = tab
                                    currentScreen = Screen.WindowsCompatibility
                                }
                            )
                        }
                    ) {
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
                                box64Status = box64Status,
                                environments = environments,
                                environmentStates = environmentStates,
                                environmentStorage = environmentStorage,
                                availableStorageBytes = diagnostics?.available_storage_bytes,
                                environmentActionMessage = environmentActionMessage,
                                wineStatus = wineStatus,
                                wineActionMessage = wineActionMessage,
                                isRefreshing = isRefreshing,
                                onOpenDrawer = { uiScope.launch { drawerState.open() } },
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
                                onOpenCompatibility = { currentScreen = Screen.Compatibility },
                                windowsOverview = windowsOverview,
                                onOpenWindowsCompatibility = { tab ->
                                    windowsCompatInitialTab = tab
                                    currentScreen = Screen.WindowsCompatibility
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
                            Screen.WindowsCompatibility -> WindowsCompatibilityScreen(
                                environments = environments,
                                selectedEnvId = selectedEnvId,
                                initialTab = windowsCompatInitialTab,
                                onSelectEnv = { selectedEnvId = it },
                                onOpenBox64 = { currentScreen = Screen.Compatibility },
                                onBack = {
                                    refreshData()
                                    currentScreen = Screen.Dashboard
                                }
                            )
                            Screen.Compatibility -> CompatibilityScreen(
                                environments = environments,
                                selectedEnvId = selectedEnvId,
                                onSelectEnv = { selectedEnvId = it },
                                onBack = {
                                    refreshData()
                                    currentScreen = Screen.Dashboard
                                }
                            )
                            Screen.Terminal -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                TerminalScreen(
                                    envId = selectedEnvId,
                                    defaultShell = defaultShell,
                                    onBack = { currentScreen = Screen.Dashboard }
                                )
                            }
                            Screen.EnvironmentDetails -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                environments.firstOrNull { it.id == selectedEnvId }?.let { env ->
                                    EnvironmentDetailsScreen(env, environmentStates[env.id], environmentStorage[env.id], onBackups = {
                                        backupSourceId = env.id
                                        currentScreen = Screen.EnvironmentBackups
                                    }) {
                                        refreshData()
                                        currentScreen = Screen.Dashboard
                                    }
                                }
                            }
                            Screen.EnvironmentBackups -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                EnvironmentBackupsScreen(
                                    source = environments.firstOrNull { it.id == backupSourceId },
                                    onBack = {
                                        currentScreen = if (backupSourceId == null) Screen.Dashboard else Screen.EnvironmentDetails
                                    },
                                    onRestored = { id ->
                                        selectedEnvId = id
                                        refreshData()
                                        currentScreen = Screen.Dashboard
                                    }
                                )
                            }
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
                            Screen.VirtualMachines -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                VirtualMachinesScreen(
                                    onOpenVmDisplay = { vncPort ->
                                        activeDisplayPort = vncPort
                                        managedDesktop = false
                                        currentScreen = Screen.DisplayViewport
                                    },
                                    onBack = { currentScreen = Screen.Dashboard }
                                )
                            }
                            Screen.Artifacts -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                ArtifactsScreen(
                                    onBack = { currentScreen = Screen.Dashboard }
                                )
                            }
                            Screen.HealthCheck -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                HealthStatusScreen(
                                    healthReport = healthReport,
                                    onBack = { currentScreen = Screen.Dashboard },
                                    onRunCheck = refreshData
                                )
                            }
                            Screen.SystemInfo -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                SystemInfoScreen(
                                    diagnostics = diagnostics,
                                    onBack = { currentScreen = Screen.Dashboard }
                                )
                            }
                            Screen.CreateEnvironment -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                CreateEnvironmentScreen(
                                    onBack = { currentScreen = Screen.Dashboard },
                                    onCreated = { id ->
                                        selectedEnvId = id
                                        refreshData()
                                        currentScreen = Screen.Terminal
                                    }
                                )
                            }
                            Screen.Settings -> Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                                SettingsScreen(
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
    }
}

@Composable
private fun StarNavigationDrawerSheet(
    currentScreen: MainActivity.Screen,
    windowsCompatTab: Int,
    engineStatus: EngineStatusData?,
    onSelect: (MainActivity.Screen) -> Unit,
    onSelectWindowsTab: (Int) -> Unit
) {
    ModalDrawerSheet(
        drawerContainerColor = SurfaceDark,
        drawerContentColor = TextPrimary,
        modifier = Modifier.widthIn(max = 300.dp)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .background(SurfaceElevated)
                .statusBarsPadding()
                .padding(horizontal = 20.dp, vertical = 20.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                StarEmblem(modifier = Modifier.size(30.dp))
                Column {
                    Text(
                        text = "Android PC",
                        style = MaterialTheme.typography.titleLarge,
                        color = TextPrimary
                    )
                    Text(
                        text = "v${engineStatus?.version ?: BuildConfig.VERSION_NAME} • ${engineStatus?.host_arch ?: "aarch64"}",
                        style = MaterialTheme.typography.labelSmall,
                        color = StarSilver
                    )
                }
            }
        }

        HorizontalDivider(color = CardBorder)

        Spacer(modifier = Modifier.height(10.dp))
        DrawerSectionHeader("WORKSPACE")
        DrawerItem("Containers", Icons.Filled.Computer, currentScreen == MainActivity.Screen.Dashboard) {
            onSelect(MainActivity.Screen.Dashboard)
        }
        DrawerItem("Desktop (Xvnc)", Icons.Filled.DesktopWindows, currentScreen == MainActivity.Screen.DisplayViewport) {
            onSelect(MainActivity.Screen.DisplayViewport)
        }
        DrawerItem("Terminal", Icons.Filled.Terminal, currentScreen == MainActivity.Screen.Terminal) {
            onSelect(MainActivity.Screen.Terminal)
        }

        Spacer(modifier = Modifier.height(8.dp))
        DrawerSectionHeader("WINDOWS COMPATIBILITY")
        DrawerItem(
            "Windows Apps",
            Icons.Filled.Apps,
            currentScreen == MainActivity.Screen.WindowsCompatibility && windowsCompatTab == 0
        ) {
            onSelectWindowsTab(0)
        }
        DrawerItem(
            "Wine & Prefixes",
            Icons.Filled.FolderCopy,
            currentScreen == MainActivity.Screen.WindowsCompatibility && windowsCompatTab == 1
        ) {
            onSelectWindowsTab(1)
        }
        DrawerItem(
            "Graphics & DXVK",
            Icons.Filled.Tune,
            currentScreen == MainActivity.Screen.WindowsCompatibility && windowsCompatTab == 2
        ) {
            onSelectWindowsTab(2)
        }
        DrawerItem("Box64 Compatibility", Icons.Filled.Memory, currentScreen == MainActivity.Screen.Compatibility) {
            onSelect(MainActivity.Screen.Compatibility)
        }

        Spacer(modifier = Modifier.height(10.dp))
        HorizontalDivider(color = CardBorder, modifier = Modifier.padding(horizontal = 16.dp))
        Spacer(modifier = Modifier.height(10.dp))

        DrawerSectionHeader("SYSTEM & TOOLS")
        DrawerItem("Virtual Machines", Icons.Filled.Dns, currentScreen == MainActivity.Screen.VirtualMachines) {
            onSelect(MainActivity.Screen.VirtualMachines)
        }
        DrawerItem("Runtime Artifacts", Icons.Filled.Download, currentScreen == MainActivity.Screen.Artifacts) {
            onSelect(MainActivity.Screen.Artifacts)
        }
        DrawerItem("Backups", Icons.Filled.Backup, currentScreen == MainActivity.Screen.EnvironmentBackups) {
            onSelect(MainActivity.Screen.EnvironmentBackups)
        }
        DrawerItem("Engine Health", Icons.Filled.HealthAndSafety, currentScreen == MainActivity.Screen.HealthCheck) {
            onSelect(MainActivity.Screen.HealthCheck)
        }
        DrawerItem("System Diagnostics", Icons.Filled.Info, currentScreen == MainActivity.Screen.SystemInfo) {
            onSelect(MainActivity.Screen.SystemInfo)
        }
        DrawerItem("Settings & Logs", Icons.Filled.Settings, currentScreen == MainActivity.Screen.Settings) {
            onSelect(MainActivity.Screen.Settings)
        }
        Spacer(modifier = Modifier.navigationBarsPadding())
    }
}

@Composable
private fun DrawerSectionHeader(title: String) {
    Text(
        text = title,
        style = MaterialTheme.typography.labelSmall,
        color = TextMuted,
        letterSpacing = 1.sp,
        modifier = Modifier.padding(horizontal = 20.dp, vertical = 6.dp)
    )
}

@Composable
private fun DrawerItem(
    label: String,
    icon: ImageVector,
    selected: Boolean,
    onClick: () -> Unit
) {
    NavigationDrawerItem(
        label = { Text(label, style = MaterialTheme.typography.bodyMedium) },
        icon = { Icon(icon, contentDescription = null, modifier = Modifier.size(20.dp)) },
        selected = selected,
        onClick = onClick,
        shape = RoundedCornerShape(10.dp),
        colors = NavigationDrawerItemDefaults.colors(
            selectedContainerColor = AccentPurple.copy(alpha = 0.22f),
            selectedIconColor = AccentCyan,
            selectedTextColor = TextPrimary,
            unselectedIconColor = TextSecondary,
            unselectedTextColor = TextSecondary
        ),
        modifier = Modifier.padding(horizontal = 12.dp, vertical = 2.dp)
    )
}
