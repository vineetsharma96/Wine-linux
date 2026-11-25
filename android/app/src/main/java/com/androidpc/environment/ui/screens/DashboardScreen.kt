package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.EngineStatusData
import com.androidpc.environment.bridge.EnvironmentConfigData
import com.androidpc.environment.bridge.HealthReportData
import com.androidpc.environment.bridge.WineStatusData
import com.androidpc.environment.ui.theme.*

@Composable
fun DashboardScreen(
    engineStatus: EngineStatusData?,
    healthReport: HealthReportData?,
    environments: List<EnvironmentConfigData>,
    environmentStates: Map<String, String>,
    environmentStorage: Map<String, Long?>,
    availableStorageBytes: Long?,
    environmentActionMessage: String?,
    wineStatus: WineStatusData?,
    wineActionMessage: String?,
    isRefreshing: Boolean,
    onRefresh: () -> Unit,
    onCreateEnvironment: () -> Unit,
    onStartEnvironment: (String) -> Unit,
    onStopEnvironment: (String) -> Unit,
    onDeleteEnvironment: (String) -> Unit,
    onRecoverEnvironment: (String, Boolean) -> Unit,
    onOpenEnvironmentTerminal: (String) -> Unit,
    onOpenEnvironmentDetails: (String) -> Unit,
    onOpenBackups: () -> Unit,
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
            title = { Text("Remove this Linux environment?") },
            text = { Text("This deletes its private files and installed packages.") },
            confirmButton = { TextButton(onClick = { pendingRemoval = null; onDeleteEnvironment(env.id) }) { Text("Remove") } },
            dismissButton = { TextButton(onClick = { pendingRemoval = null }) { Text("Cancel") } }
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
                        singleLine = true,
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = AccentCyan,
                            unfocusedBorderColor = CardBorder
                        )
                    )
                    OutlinedTextField(
                        value = exeArgsInput,
                        onValueChange = { exeArgsInput = it },
                        label = { Text("Arguments (space-separated)", color = TextSecondary) },
                        singleLine = true,
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = AccentCyan,
                            unfocusedBorderColor = CardBorder
                        )
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

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        text = "Android PC",
                        style = MaterialTheme.typography.titleMedium,
                        color = AccentCyan,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = "Dashboard",
                        style = MaterialTheme.typography.headlineLarge,
                        color = TextPrimary,
                        fontWeight = FontWeight.Bold
                    )
                }

                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedButton(
                        onClick = onOpenSettings,
                        border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text("Settings", color = TextPrimary, style = MaterialTheme.typography.bodyMedium)
                    }
                    Button(
                        onClick = onRefresh,
                        enabled = !isRefreshing,
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                        border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text(
                            text = if (isRefreshing) "Polling..." else "Poll Rust",
                            color = TextPrimary,
                            style = MaterialTheme.typography.bodyMedium
                        )
                    }
                }
            }
        }

        // Engine Status Card
        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "CORE ENGINE STATUS",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted,
                            letterSpacing = 1.sp
                        )

                        val stateColor = when (engineStatus?.state) {
                            "Ready" -> StatusGreen
                            "Degraded" -> StatusAmber
                            else -> AccentCyan
                        }

                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Box(
                                modifier = Modifier
                                    .size(8.dp)
                                    .background(stateColor, RoundedCornerShape(4.dp))
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = engineStatus?.state ?: "Connecting...",
                                style = MaterialTheme.typography.bodyMedium,
                                color = stateColor,
                                fontWeight = FontWeight.Bold
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(14.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        MetricItem("Version", engineStatus?.version ?: "0.1.0-alpha.0")
                        MetricItem("Arch", engineStatus?.host_arch ?: "aarch64")
                        MetricItem("Envs", "${environments.size}")
                        MetricItem("Uptime", "${engineStatus?.uptime_seconds ?: 0}s")
                    }
                }
            }
        }

        // Health Status Summary Card
        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "SYSTEM HEALTH REPORT",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted,
                            letterSpacing = 1.sp
                        )

                        val statusText = healthReport?.status ?: "Healthy"
                        val statusColor = when (statusText) {
                            "Healthy" -> StatusGreen
                            "Degraded" -> StatusAmber
                            else -> StatusRed
                        }

                        Text(
                            text = statusText,
                            style = MaterialTheme.typography.bodyMedium,
                            color = statusColor,
                            fontWeight = FontWeight.Bold
                        )
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    healthReport?.subsystems?.take(4)?.forEach { sub ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 4.dp),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = sub.name,
                                style = MaterialTheme.typography.bodyMedium,
                                color = TextSecondary
                            )
                            Text(
                                text = "${sub.level} • ${sub.latency_ms}ms",
                                style = MaterialTheme.typography.labelSmall,
                                color = if (sub.level == "Healthy") StatusGreen else StatusAmber
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    TextButton(
                        onClick = onOpenHealthCheck,
                        modifier = Modifier.align(Alignment.End)
                    ) {
                        Text("View Subsystem Details →", color = AccentCyan)
                    }
                }
            }
        }

        // Provisioned Environments List
        if (environments.isNotEmpty()) {
            item {
                Text(
                    text = "PROVISIONED ENVIRONMENTS",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextMuted,
                    letterSpacing = 1.sp
                )
            }
            environmentActionMessage?.let { message ->
                item { Text(message, color = TextSecondary, style = MaterialTheme.typography.bodySmall) }
            }
            items(environments, key = { it.id }) { env ->
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Column(modifier = Modifier.padding(14.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column {
                                val distro = env.distribution.replaceFirstChar(Char::uppercase)
                                Text(env.display_name ?: distro, style = MaterialTheme.typography.titleMedium, color = TextPrimary, fontWeight = FontWeight.Bold)
                                Text(
                                    "$distro ${env.version} • ${env.architecture} • ${environmentStates[env.id] ?: "Checking"}",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = TextSecondary
                                )
                                environmentStorage[env.id]?.let { bytes ->
                                    Text("${bytes / (1024 * 1024)} MiB", style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                                }
                                env.last_started_at_epoch_ms?.let { timestamp ->
                                    val lastStarted = java.text.DateFormat.getDateTimeInstance(
                                        java.text.DateFormat.SHORT, java.text.DateFormat.SHORT
                                    ).format(java.util.Date(timestamp))
                                    Text("Last started $lastStarted", style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                                }
                            }
                            Row {
                                TextButton(onClick = { onOpenEnvironmentDetails(env.id) }) { Text("Details") }
                                TextButton(onClick = { onOpenEnvironmentTerminal(env.id) }) { Text("Terminal") }
                            }
                        }
                        Row(horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                                TextButton(onClick = { onStartEnvironment(env.id) }) {
                                    Text("Start", color = StatusGreen)
                                }
                                TextButton(onClick = { onStopEnvironment(env.id) }) {
                                    Text("Stop Linux", color = StatusAmber)
                                }
                                TextButton(onClick = { onRecoverEnvironment(env.id, true) }) { Text("Restart") }
                                TextButton(onClick = { onRecoverEnvironment(env.id, false) }) { Text("Repair") }
                                TextButton(onClick = { pendingRemoval = env }, enabled = environmentStates[env.id] != "Running") {
                                    Text("Remove", color = StatusRed)
                                }
                        }
                    }
                }
            }
        }

        item {
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                val running = environmentStates.values.count { it == "Running" }
                val available = availableStorageBytes?.let { " · ${it / (1024 * 1024)} MiB available" }.orEmpty()
                Text("$running running$available", color = TextSecondary, style = MaterialTheme.typography.bodySmall)
                TextButton(onClick = onOpenBackups) { Text("All backups", color = AccentCyan) }
            }
        }

        // Windows Compatibility (Wine + Box64)
        item {
            val wineStateText = wineStatus?.state ?: "Uninitialized"
            val wineBadgeColor = when (wineStateText) {
                "Running", "Ready" -> StatusGreen
                "InitializingPrefix" -> StatusAmber
                "Terminated" -> TextMuted
                else -> AccentCyan
            }

            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "WINDOWS COMPATIBILITY (WINE + BOX64)",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted,
                            letterSpacing = 1.sp
                        )
                        Surface(
                            shape = RoundedCornerShape(4.dp),
                            color = wineBadgeColor.copy(alpha = 0.15f)
                        ) {
                            Text(
                                text = wineStateText.uppercase(),
                                color = wineBadgeColor,
                                style = MaterialTheme.typography.labelSmall,
                                fontWeight = FontWeight.Bold,
                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        MetricItem(label = "Wine Arch", value = wineStatus?.arch ?: "Win64")
                        MetricItem(
                            label = "Box64 Dynarec",
                            value = if (wineStatus?.box64_dynarec != false) "Active (ARM64)" else "Disabled"
                        )
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        MetricItem(
                            label = "Prefix C:",
                            value = if (wineStatus?.prefix_exists == true) "Initialized" else (wineStatus?.prefix_path?.ifEmpty { "~/.wine" } ?: "~/.wine")
                        )
                        MetricItem(
                            label = "Wineserver",
                            value = if (wineStatus?.wineserver_running == true) "Running (${wineStatus.active_wine_pids.size} PID)" else "Idle"
                        )
                    }

                    wineActionMessage?.let { msg ->
                        Spacer(modifier = Modifier.height(10.dp))
                        Text(
                            text = msg,
                            style = MaterialTheme.typography.bodySmall,
                            color = AccentCyan
                        )
                    }

                    Spacer(modifier = Modifier.height(14.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Button(
                            onClick = onInitWinePrefix,
                            modifier = Modifier
                                .weight(1f)
                                .height(38.dp),
                            colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                            border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                            shape = RoundedCornerShape(8.dp)
                        ) {
                            Text("Init C: Drive", style = MaterialTheme.typography.labelMedium, color = AccentCyan)
                        }

                        Button(
                            onClick = { showRunExeDialog = true },
                            modifier = Modifier
                                .weight(1f)
                                .height(38.dp),
                            colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                            border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                            shape = RoundedCornerShape(8.dp)
                        ) {
                            Text("Run .exe", style = MaterialTheme.typography.labelMedium, color = TextPrimary)
                        }

                        if (wineStatus?.wineserver_running == true) {
                            Button(
                                onClick = onKillWineSession,
                                modifier = Modifier.height(38.dp),
                                colors = ButtonDefaults.buttonColors(containerColor = StatusRed.copy(alpha = 0.2f)),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text("Kill", style = MaterialTheme.typography.labelMedium, color = StatusRed)
                            }
                        }
                    }
                }
            }
        }

        // Quick Actions
        item {
            Text(
                text = "ACTIONS",
                style = MaterialTheme.typography.labelSmall,
                color = TextMuted,
                letterSpacing = 1.sp
            )

            val termuxContext = androidx.compose.ui.platform.LocalContext.current
            var showTermuxDialog by remember { mutableStateOf(false) }
            val termuxInstalled = remember(termuxContext) {
                try {
                    termuxContext.packageManager.getPackageInfo("com.termux.x11", 0)
                    true
                } catch (_: Exception) {
                    false
                }
            }

            Spacer(modifier = Modifier.height(8.dp))
            OutlinedButton(
                onClick = {
                    if (termuxInstalled) {
                        val intent = termuxContext.packageManager.getLaunchIntentForPackage("com.termux.x11")
                        if (intent != null) termuxContext.startActivity(intent) else showTermuxDialog = true
                    } else {
                        showTermuxDialog = true
                    }
                },
                modifier = Modifier.fillMaxWidth().height(40.dp),
                shape = RoundedCornerShape(10.dp)
            ) {
                Text(
                    text = if (termuxInstalled) "Backend B: Launch Termux:X11" else "Backend B: Termux:X11 Setup",
                    color = TextSecondary
                )
            }

            if (showTermuxDialog) {
                AlertDialog(
                    onDismissRequest = { showTermuxDialog = false },
                    title = { Text("Termux:X11 External Backend", color = TextPrimary) },
                    text = {
                        Text(
                            if (termuxInstalled) {
                                "Termux:X11 is installed. Start Termux:X11 and set DISPLAY=:0 inside your Linux environment to route X11 output externally."
                            } else {
                                "com.termux.x11 is not installed on this device.\n\nRequirements:\n1. Install the Termux:X11 Android APK.\n2. Start Termux:X11 and share its X11 socket/DISPLAY=:0.\n3. Note: Android app sandboxes isolate /data/data/com.termux unless explicitly bridged; use Integrated Xvnc for self-contained rootless desktop."
                            },
                            color = TextSecondary
                        )
                    },
                    confirmButton = {
                        TextButton(onClick = { showTermuxDialog = false }) { Text("OK") }
                    }
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                Button(
                    onClick = onOpenDisplayViewport,
                    modifier = Modifier
                        .weight(1f)
                        .height(48.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = AccentPurple),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        text = "Desktop (Xvnc)",
                        color = Color.White,
                        fontWeight = FontWeight.Bold
                    )
                }

                Button(
                    onClick = onOpenTerminal,
                    modifier = Modifier
                        .weight(1f)
                        .height(48.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                    border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        text = "PTY Terminal",
                        color = AccentCyan,
                        fontWeight = FontWeight.Bold
                    )
                }
            }

            Spacer(modifier = Modifier.height(10.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Button(
                    onClick = onCreateEnvironment,
                    modifier = Modifier
                        .weight(1f)
                        .height(48.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        text = "+ New Environment",
                        color = BgDark,
                        fontWeight = FontWeight.Bold
                    )
                }

                Button(
                    onClick = onOpenSystemInfo,
                    modifier = Modifier
                        .weight(1f)
                        .height(48.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                    border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        text = "Diagnostics",
                        color = TextPrimary
                    )
                }
            }

            Spacer(modifier = Modifier.height(10.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                Button(
                    onClick = onOpenVirtualMachines,
                    modifier = Modifier
                        .weight(1f)
                        .height(46.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                    border = androidx.compose.foundation.BorderStroke(1.dp, AccentPurple),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        text = "QEMU / KVM VMs",
                        color = TextPrimary,
                        fontWeight = FontWeight.SemiBold
                    )
                }

                Button(
                    onClick = onOpenArtifacts,
                    modifier = Modifier
                        .weight(1f)
                        .height(46.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = SurfaceContainer),
                    border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        text = "Artifacts",
                        color = AccentCyan,
                        fontWeight = FontWeight.SemiBold
                    )
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
            style = MaterialTheme.typography.bodyLarge,
            color = TextPrimary,
            fontWeight = FontWeight.SemiBold
        )
    }
}
