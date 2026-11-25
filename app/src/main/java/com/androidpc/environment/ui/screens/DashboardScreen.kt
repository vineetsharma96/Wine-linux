package com.androidpc.environment.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.DeleteOutline
import androidx.compose.material.icons.filled.DesktopWindows
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material.icons.filled.Terminal
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.Box64InstallationStatusData
import com.androidpc.environment.bridge.EngineStatusData
import com.androidpc.environment.bridge.EnvironmentConfigData
import com.androidpc.environment.bridge.HealthReportData
import com.androidpc.environment.bridge.WineStatusData
import com.androidpc.environment.bridge.WindowsCompatibilityOverviewData
import com.androidpc.environment.ui.theme.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    engineStatus: EngineStatusData?,
    healthReport: HealthReportData?,
    box64Status: Box64InstallationStatusData? = null,
    environments: List<EnvironmentConfigData>,
    environmentStates: Map<String, String>,
    environmentStorage: Map<String, Long?>,
    availableStorageBytes: Long?,
    environmentActionMessage: String?,
    wineStatus: WineStatusData?,
    wineActionMessage: String?,
    isRefreshing: Boolean,
    onOpenDrawer: () -> Unit = {},
    onRefresh: () -> Unit,
    onCreateEnvironment: () -> Unit,
    onStartEnvironment: (String) -> Unit,
    onStopEnvironment: (String) -> Unit,
    onDeleteEnvironment: (String) -> Unit,
    onRecoverEnvironment: (String, Boolean) -> Unit,
    onOpenEnvironmentTerminal: (String) -> Unit,
    onOpenEnvironmentDetails: (String) -> Unit,
    onOpenBackups: () -> Unit,
    onOpenCompatibility: () -> Unit = {},
    windowsOverview: WindowsCompatibilityOverviewData? = null,
    onOpenWindowsCompatibility: (Int) -> Unit = {},
    onInitWinePrefix: () -> Unit,
    onLaunchWineApp: (String, List<String>) -> Unit,
    onKillWineSession: () -> Unit,
    onOpenHealthCheck: () -> Unit,
    onOpenSystemInfo: () -> Unit,
    onOpenDisplayViewport: () -> Unit = {},
    onOpenTerminal: () -> Unit = {},
    onOpenVirtualMachines: () -> Unit = {},
    onOpenArtifacts: () -> Unit = {},
    onOpenSettings: () -> Unit = {}
) {
    var showRunExeDialog by remember { mutableStateOf(false) }
    var exePathInput by remember { mutableStateOf("C:\\windows\\system32\\notepad.exe") }
    var exeArgsInput by remember { mutableStateOf("") }
    var pendingRemoval by remember { mutableStateOf<EnvironmentConfigData?>(null) }

    pendingRemoval?.let { env ->
        AlertDialog(
            onDismissRequest = { pendingRemoval = null },
            containerColor = SurfaceDark,
            title = { Text("Remove this Linux container?", color = TextPrimary) },
            text = { Text("This deletes its private files and installed packages.", color = TextSecondary) },
            confirmButton = {
                TextButton(onClick = { pendingRemoval = null; onDeleteEnvironment(env.id) }) {
                    Text("Remove", color = StatusRed)
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingRemoval = null }) { Text("Cancel", color = TextSecondary) }
            }
        )
    }

    if (showRunExeDialog) {
        AlertDialog(
            onDismissRequest = { showRunExeDialog = false },
            containerColor = SurfaceDark,
            title = { Text("Run Windows Application (Box64 + Wine)", color = TextPrimary) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    OutlinedTextField(
                        value = exePathInput,
                        onValueChange = { exePathInput = it },
                        label = { Text("Executable Path (Windows or Linux)", color = TextSecondary) },
                        singleLine = true
                    )
                    OutlinedTextField(
                        value = exeArgsInput,
                        onValueChange = { exeArgsInput = it },
                        label = { Text("Arguments (space-separated)", color = TextSecondary) },
                        singleLine = true
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("winecfg", "cmd.exe", "notepad.exe").forEach { preset ->
                            TextButton(onClick = { exePathInput = preset }) {
                                Text(preset, color = AccentCyan, style = MaterialTheme.typography.labelSmall)
                            }
                        }
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        val parsedArgs = exeArgsInput.trim().split(" ").filter { it.isNotEmpty() }
                        onLaunchWineApp(exePathInput.trim(), parsedArgs)
                        showRunExeDialog = false
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = AccentCyan)
                ) {
                    Text("Launch", color = BgDark, fontWeight = FontWeight.Bold)
                }
            },
            dismissButton = {
                TextButton(onClick = { showRunExeDialog = false }) {
                    Text("Cancel", color = TextSecondary)
                }
            }
        )
    }

    Scaffold(
        containerColor = BgDark,
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = SurfaceDark,
                    titleContentColor = TextPrimary
                ),
                navigationIcon = {
                    IconButton(
                        onClick = onOpenDrawer,
                        modifier = Modifier.testTag("drawer_menu_button")
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Menu,
                            contentDescription = "Open Navigation Drawer",
                            tint = TextPrimary
                        )
                    }
                },
                title = {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        StarEmblem(modifier = Modifier.size(22.dp))
                        Column {
                            Text(
                                text = "Containers",
                                style = MaterialTheme.typography.titleLarge,
                                color = TextPrimary
                            )
                            Text(
                                text = "Android PC • ${engineStatus?.host_arch ?: "aarch64"}",
                                style = MaterialTheme.typography.labelSmall,
                                color = TextSecondary
                            )
                        }
                    }
                },
                actions = {
                    IconButton(
                        onClick = onRefresh,
                        enabled = !isRefreshing,
                        modifier = Modifier.testTag("dashboard_refresh_button")
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Refresh,
                            contentDescription = "Refresh Engine State",
                            tint = if (isRefreshing) TextMuted else AccentCyan
                        )
                    }
                    IconButton(
                        onClick = onOpenSettings,
                        modifier = Modifier.testTag("dashboard_settings_button")
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Settings,
                            contentDescription = "Settings",
                            tint = TextSecondary
                        )
                    }
                }
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = onCreateEnvironment,
                containerColor = AccentCyan,
                contentColor = BgDark,
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier.testTag("create_container_fab"),
                icon = { Icon(Icons.Filled.Add, contentDescription = null) },
                text = { Text("New Container", fontWeight = FontWeight.Bold) }
            )
        }
    ) { innerPadding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(horizontal = 16.dp),
            contentPadding = PaddingValues(top = 14.dp, bottom = 88.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            // Star Plus Compact Telemetry Strip
            item {
                val stateColor = when (engineStatus?.state) {
                    "Ready" -> StatusGreen
                    "Degraded" -> StatusAmber
                    else -> AccentCyan
                }
                val runningCount = environmentStates.values.count { it == "Running" }
                val storageLabel = availableStorageBytes?.let { "${it / (1024 * 1024)} MiB free" } ?: "—"

                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(8.dp)
                                        .clip(CircleShape)
                                        .background(stateColor)
                                )
                                Text(
                                    text = "RUST ENGINE ${engineStatus?.state?.uppercase() ?: "CONNECTING"}",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = stateColor,
                                    letterSpacing = 0.8.sp
                                )
                            }
                            Text(
                                text = "v${engineStatus?.version ?: "0.1.0-alpha.0"}",
                                style = MaterialTheme.typography.labelSmall,
                                color = TextMuted
                            )
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            MetricItem("Containers", "${environments.size} ($runningCount active)")
                            MetricItem("Storage", storageLabel)
                            MetricItem(
                                "Box64",
                                if (box64Status?.installed == true) "v${box64Status.pinned_version}" else "Pending"
                            )
                        }
                    }
                }
            }

            // Phase 4 — Complete Windows Compatibility Engine Summary Card
            item {
                val effectiveBox64 = windowsOverview?.box64 ?: box64Status
                val wineInfo = windowsOverview?.wine
                val gfxInfo = windowsOverview?.graphics
                val activeBackendReport = gfxInfo?.backends?.firstOrNull { it.kind == gfxInfo.effective_backend }
                val gfxStage = activeBackendReport?.stage?.uppercase()?.replace('_', ' ') ?: "SOFTWARE"
                val gfxCaps = buildList {
                    if (activeBackendReport?.supports_opengl != false) add("OpenGL")
                    if (activeBackendReport?.supports_vulkan == true) add("Vulkan")
                    if (gfxInfo?.dx_translation?.dxvk_eligible == true) add("DXVK")
                    if (gfxInfo?.dx_translation?.vkd3d_eligible == true) add("VKD3D")
                }.joinToString(" • ").ifEmpty { "OpenGL (Software)" }
                val runtimeErrors = windowsOverview?.runtime_errors.orEmpty()

                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, AccentPurple.copy(alpha = 0.55f), RoundedCornerShape(14.dp))
                        .testTag("dashboard_windows_compat_card"),
                    colors = CardDefaults.cardColors(containerColor = SurfaceElevated),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(14.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(
                                modifier = Modifier.weight(1f),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(10.dp)
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(38.dp)
                                        .clip(RoundedCornerShape(10.dp))
                                        .background(AccentPurple.copy(alpha = 0.2f)),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Icon(Icons.Filled.Memory, contentDescription = null, tint = AccentCyan)
                                }
                                Column {
                                    Text(
                                        text = "Windows Compatibility Engine",
                                        style = MaterialTheme.typography.titleMedium,
                                        color = TextPrimary,
                                        fontWeight = FontWeight.SemiBold
                                    )
                                    Text(
                                        text = "Box64 • Wine WoW64 • Turnip / VirGL / Software",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = TextSecondary
                                    )
                                }
                            }
                            StatusPill(
                                text = if (effectiveBox64?.installed == true && wineInfo?.installed == true) "READY" else "STAGED",
                                color = if (effectiveBox64?.installed == true && wineInfo?.installed == true) StatusGreen else AccentCyan
                            )
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            MetricItem(
                                "Box64",
                                if (effectiveBox64?.installed == true) "v${effectiveBox64.pinned_version} Ready" else "v${effectiveBox64?.pinned_version ?: "0.4.4"} Pending"
                            )
                            MetricItem(
                                "Wine",
                                if (wineInfo?.installed == true) "v${wineInfo.pinned_version} WoW64" else "v${wineInfo?.pinned_version ?: "11.0"} Pending"
                            )
                            MetricItem(
                                "Prefixes / Apps",
                                "${windowsOverview?.prefix_count ?: 0} / ${windowsOverview?.installed_apps_count ?: 0}"
                            )
                        }

                        Surface(
                            color = SurfaceDark,
                            shape = RoundedCornerShape(10.dp),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Column(
                                modifier = Modifier.padding(10.dp),
                                verticalArrangement = Arrangement.spacedBy(4.dp)
                            ) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    Text(
                                        text = "Graphics: ${(gfxInfo?.effective_backend ?: "software").uppercase()} ($gfxStage)",
                                        style = MaterialTheme.typography.labelMedium,
                                        color = AccentCyan,
                                        fontWeight = FontWeight.SemiBold
                                    )
                                    Text(
                                        text = "Active procs: ${windowsOverview?.active_processes_count ?: 0}",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = TextSecondary
                                    )
                                }
                                Text(
                                    text = "Capabilities: $gfxCaps • DX: ${gfxInfo?.dx_translation?.effective_choice ?: "wine_d3d_open_gl"}",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = TextSecondary
                                )
                                if (runtimeErrors.isNotEmpty()) {
                                    Text(
                                        text = "Runtime notices (${runtimeErrors.size}): ${runtimeErrors.first()}",
                                        style = MaterialTheme.typography.bodySmall,
                                        color = StatusAmber
                                    )
                                }
                            }
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Button(
                                onClick = { onOpenWindowsCompatibility(0) },
                                modifier = Modifier
                                    .weight(1f)
                                    .height(42.dp)
                                    .testTag("dashboard_open_windows_apps_button"),
                                colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                                shape = RoundedCornerShape(10.dp)
                            ) {
                                Text("Windows Apps", color = BgDark, fontWeight = FontWeight.Bold, style = MaterialTheme.typography.labelMedium)
                            }
                            OutlinedButton(
                                onClick = { onOpenWindowsCompatibility(2) },
                                modifier = Modifier
                                    .weight(1f)
                                    .height(42.dp)
                                    .testTag("dashboard_open_graphics_button"),
                                border = BorderStroke(1.dp, CardBorder),
                                shape = RoundedCornerShape(10.dp)
                            ) {
                                Text("Graphics", color = TextPrimary, style = MaterialTheme.typography.labelMedium)
                            }
                            OutlinedButton(
                                onClick = onOpenCompatibility,
                                modifier = Modifier
                                    .height(42.dp)
                                    .testTag("dashboard_box64_card"),
                                border = BorderStroke(1.dp, CardBorder),
                                shape = RoundedCornerShape(10.dp)
                            ) {
                                Text("Box64", color = AccentCyan, style = MaterialTheme.typography.labelMedium)
                            }
                        }
                    }
                }
            }

            // Containers Header
            item {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = "LINUX CONTAINERS",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextMuted,
                        letterSpacing = 1.sp
                    )
                    TextButton(onClick = onOpenBackups) {
                        Text("Backups", color = AccentCyan, style = MaterialTheme.typography.labelMedium)
                    }
                }
            }

            environmentActionMessage?.let { message ->
                item {
                    Surface(
                        color = SurfaceContainer,
                        shape = RoundedCornerShape(8.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text(
                            text = message,
                            color = AccentCyan,
                            style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.padding(10.dp)
                        )
                    }
                }
            }

            if (environments.isEmpty()) {
                item {
                    Card(
                        modifier = Modifier
                            .fillMaxWidth()
                            .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                        colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                        shape = RoundedCornerShape(14.dp)
                    ) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(24.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(10.dp)
                        ) {
                            Icon(
                                imageVector = Icons.Filled.Computer,
                                contentDescription = null,
                                tint = TextMuted,
                                modifier = Modifier.size(36.dp)
                            )
                            Text(
                                text = "No Containers Installed",
                                style = MaterialTheme.typography.titleMedium,
                                color = TextPrimary
                            )
                            Text(
                                text = "Deploy an ARM64 Ubuntu 26.04 or Debian 13 rootless container to launch XFCE desktop, terminal, and Box64 x86-64 apps.",
                                style = MaterialTheme.typography.bodySmall,
                                color = TextSecondary
                            )
                            OutlinedButton(
                                onClick = onCreateEnvironment,
                                border = BorderStroke(1.dp, AccentCyan),
                                shape = RoundedCornerShape(10.dp)
                            ) {
                                Text("Deploy Container", color = AccentCyan)
                            }
                        }
                    }
                }
            } else {
                items(environments, key = { it.id }) { env ->
                    StarContainerCard(
                        env = env,
                        state = environmentStates[env.id] ?: "Stopped",
                        storageBytes = environmentStorage[env.id],
                        onStart = { onStartEnvironment(env.id) },
                        onStop = { onStopEnvironment(env.id) },
                        onDesktop = onOpenDisplayViewport,
                        onTerminal = { onOpenEnvironmentTerminal(env.id) },
                        onDetails = { onOpenEnvironmentDetails(env.id) },
                        onRestart = { onRecoverEnvironment(env.id, true) },
                        onRepair = { onRecoverEnvironment(env.id, false) },
                        onRemove = { pendingRemoval = env }
                    )
                }
            }

            // Quick Launch Row (Desktop, Terminal, VMs, Artifacts)
            item {
                Text(
                    text = "WORKSPACE TOOLS",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextMuted,
                    letterSpacing = 1.sp
                )
            }

            item {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Button(
                            onClick = onOpenDisplayViewport,
                            modifier = Modifier.weight(1f).height(48.dp).testTag("open_desktop_button"),
                            colors = ButtonDefaults.buttonColors(containerColor = AccentPurple),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Icon(Icons.Filled.DesktopWindows, contentDescription = null, tint = Color.White, modifier = Modifier.size(18.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("Desktop", color = Color.White, fontWeight = FontWeight.SemiBold)
                        }
                        Button(
                            onClick = onOpenTerminal,
                            modifier = Modifier.weight(1f).height(48.dp).testTag("open_terminal_button"),
                            colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                            border = BorderStroke(1.dp, CardBorder),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Icon(Icons.Filled.Terminal, contentDescription = null, tint = AccentCyan, modifier = Modifier.size(18.dp))
                            Spacer(modifier = Modifier.width(6.dp))
                            Text("Terminal", color = AccentCyan, fontWeight = FontWeight.SemiBold)
                        }
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        OutlinedButton(
                            onClick = onOpenVirtualMachines,
                            modifier = Modifier.weight(1f).height(44.dp),
                            border = BorderStroke(1.dp, CardBorder),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Text("QEMU VMs", color = TextPrimary)
                        }
                        OutlinedButton(
                            onClick = onOpenArtifacts,
                            modifier = Modifier.weight(1f).height(44.dp),
                            border = BorderStroke(1.dp, CardBorder),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Text("Artifacts", color = AccentCyan)
                        }
                    }
                }
            }

            // Existing System Health Summary Card
            item {
                val statusText = healthReport?.status ?: "Healthy"
                val statusColor = when (statusText) {
                    "Healthy" -> StatusGreen
                    "Degraded" -> StatusAmber
                    else -> StatusRed
                }
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                text = "SYSTEM HEALTH",
                                style = MaterialTheme.typography.labelSmall,
                                color = TextMuted,
                                letterSpacing = 1.sp
                            )
                            StatusPill(statusText.uppercase(), statusColor)
                        }
                        healthReport?.subsystems?.take(4)?.forEach { sub ->
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text(sub.name, style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                                Text(
                                    "${sub.level} • ${sub.latency_ms}ms",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = if (sub.level == "Healthy") StatusGreen else StatusAmber
                                )
                            }
                        }
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.End
                        ) {
                            TextButton(onClick = onOpenSystemInfo) {
                                Text("Diagnostics", color = TextSecondary)
                            }
                            TextButton(onClick = onOpenHealthCheck) {
                                Text("Subsystems →", color = AccentCyan)
                            }
                        }
                    }
                }
            }

            // Existing Wine Prefix Card (Preserved from existing app, staged behind Box64)
            item {
                val wineStateText = wineStatus?.state ?: "Uninitialized"
                val wineBadgeColor = when (wineStateText) {
                    "Running", "Ready" -> StatusGreen
                    "InitializingPrefix" -> StatusAmber
                    "Terminated" -> TextMuted
                    else -> AccentCyan
                }
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                text = "WINE RUNTIME (STAGED)",
                                style = MaterialTheme.typography.labelSmall,
                                color = TextMuted,
                                letterSpacing = 1.sp
                            )
                            StatusPill(wineStateText.uppercase(), wineBadgeColor)
                        }
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            MetricItem("Arch", wineStatus?.arch ?: "Win64")
                            MetricItem("Prefix", if (wineStatus?.prefix_exists == true) "Ready" else "~/.wine")
                            MetricItem("Wineserver", if (wineStatus?.wineserver_running == true) "Active" else "Idle")
                        }
                        wineActionMessage?.let { msg ->
                            Text(msg, style = MaterialTheme.typography.bodySmall, color = AccentCyan)
                        }
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            OutlinedButton(
                                onClick = onInitWinePrefix,
                                modifier = Modifier.weight(1f),
                                border = BorderStroke(1.dp, CardBorder),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text("Init Prefix", color = AccentCyan, style = MaterialTheme.typography.labelMedium)
                            }
                            OutlinedButton(
                                onClick = { showRunExeDialog = true },
                                modifier = Modifier.weight(1f),
                                border = BorderStroke(1.dp, CardBorder),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text("Run .exe", color = TextPrimary, style = MaterialTheme.typography.labelMedium)
                            }
                            if (wineStatus?.wineserver_running == true) {
                                TextButton(onClick = onKillWineSession) {
                                    Text("Kill", color = StatusRed)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun StarContainerCard(
    env: EnvironmentConfigData,
    state: String,
    storageBytes: Long?,
    onStart: () -> Unit,
    onStop: () -> Unit,
    onDesktop: () -> Unit,
    onTerminal: () -> Unit,
    onDetails: () -> Unit,
    onRestart: () -> Unit,
    onRepair: () -> Unit,
    onRemove: () -> Unit
) {
    var menuExpanded by remember { mutableStateOf(false) }
    val isRunning = state == "Running"
    val distro = env.distribution.replaceFirstChar(Char::uppercase)

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .border(
                1.dp,
                if (isRunning) StatusGreen.copy(alpha = 0.55f) else CardBorder,
                RoundedCornerShape(14.dp)
            )
            .testTag("container_card_${env.id}"),
        colors = CardDefaults.cardColors(containerColor = SurfaceDark),
        shape = RoundedCornerShape(14.dp)
    ) {
        Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(
                    modifier = Modifier.weight(1f),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .size(44.dp)
                            .clip(RoundedCornerShape(12.dp))
                            .background(SurfaceContainer)
                            .border(1.dp, CardBorderActive, RoundedCornerShape(12.dp)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Computer,
                            contentDescription = null,
                            tint = if (isRunning) StatusGreen else AccentCyan
                        )
                    }
                    Column {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text(
                                text = env.display_name ?: distro,
                                style = MaterialTheme.typography.titleMedium,
                                color = TextPrimary,
                                fontWeight = FontWeight.Bold
                            )
                            StatusPill(
                                text = state.uppercase(),
                                color = if (isRunning) StatusGreen else TextSecondary
                            )
                        }
                        Spacer(modifier = Modifier.height(2.dp))
                        val sizePart = storageBytes?.let { " • ${it / (1024 * 1024)} MiB" }.orEmpty()
                        Text(
                            text = "$distro ${env.version} • ${env.architecture}$sizePart",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                    }
                }

                Box {
                    IconButton(onClick = { menuExpanded = true }) {
                        Icon(Icons.Filled.MoreVert, contentDescription = "Container Actions", tint = TextSecondary)
                    }
                    DropdownMenu(
                        expanded = menuExpanded,
                        onDismissRequest = { menuExpanded = false },
                        containerColor = SurfaceElevated
                    ) {
                        DropdownMenuItem(
                            text = { Text("Details & Packages", color = TextPrimary) },
                            leadingIcon = { Icon(Icons.Filled.Info, contentDescription = null, tint = AccentCyan) },
                            onClick = { menuExpanded = false; onDetails() }
                        )
                        DropdownMenuItem(
                            text = { Text("Restart Container", color = TextPrimary) },
                            onClick = { menuExpanded = false; onRestart() }
                        )
                        DropdownMenuItem(
                            text = { Text("Repair Guest Config", color = TextPrimary) },
                            onClick = { menuExpanded = false; onRepair() }
                        )
                        DropdownMenuItem(
                            text = { Text("Remove Container", color = if (isRunning) TextMuted else StatusRed) },
                            leadingIcon = { Icon(Icons.Filled.DeleteOutline, contentDescription = null, tint = if (isRunning) TextMuted else StatusRed) },
                            enabled = !isRunning,
                            onClick = { menuExpanded = false; onRemove() }
                        )
                    }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Button(
                    onClick = if (isRunning) onStop else onStart,
                    modifier = Modifier.weight(1f).height(42.dp),
                    colors = ButtonDefaults.buttonColors(
                        containerColor = if (isRunning) StatusAmber.copy(alpha = 0.2f) else AccentCyan
                    ),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Icon(
                        imageVector = if (isRunning) Icons.Filled.Stop else Icons.Filled.PlayArrow,
                        contentDescription = null,
                        tint = if (isRunning) StatusAmber else BgDark,
                        modifier = Modifier.size(18.dp)
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(
                        text = if (isRunning) "Stop" else "Start",
                        color = if (isRunning) StatusAmber else BgDark,
                        fontWeight = FontWeight.Bold
                    )
                }
                OutlinedButton(
                    onClick = onDesktop,
                    modifier = Modifier.weight(1f).height(42.dp),
                    border = BorderStroke(1.dp, CardBorder),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text("Desktop", color = TextPrimary, style = MaterialTheme.typography.labelMedium)
                }
                OutlinedButton(
                    onClick = onTerminal,
                    modifier = Modifier.weight(1f).height(42.dp),
                    border = BorderStroke(1.dp, CardBorder),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text("Terminal", color = AccentCyan, style = MaterialTheme.typography.labelMedium)
                }
            }
        }
    }
}

@Composable
fun MetricItem(label: String, value: String) {
    Column {
        Text(text = label, style = MaterialTheme.typography.labelSmall, color = TextMuted)
        Spacer(modifier = Modifier.height(2.dp))
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            color = TextPrimary,
            fontWeight = FontWeight.SemiBold
        )
    }
}
