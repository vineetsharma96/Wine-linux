package com.androidpc.environment.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Apps
import androidx.compose.material.icons.filled.DeleteOutline
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.TrackedWindowsProcessData
import com.androidpc.environment.bridge.WindowsAppProfileData
import com.androidpc.environment.bridge.WinePrefixMetadataData
import com.androidpc.environment.ui.theme.*

@Composable
internal fun WindowsAppsTabContent(
    selectedEnvId: String,
    apps: List<WindowsAppProfileData>,
    prefixes: List<WinePrefixMetadataData>,
    processes: List<TrackedWindowsProcessData>,
    onRegisterApp: (WindowsAppProfileData) -> Unit,
    onLaunchApp: (WindowsAppProfileData) -> Unit,
    onStopApp: (String, Boolean) -> Unit,
    onRemoveApp: (String) -> Unit
) {
    var showAddDialog by remember { mutableStateOf(false) }

    if (showAddDialog) {
        AddWindowsAppDialog(
            selectedEnvId = selectedEnvId,
            prefixes = prefixes,
            onDismiss = { showAddDialog = false },
            onConfirm = { profile ->
                onRegisterApp(profile)
                showAddDialog = false
            }
        )
    }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = "WINDOWS APPLICATION LIBRARY",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextMuted,
                    letterSpacing = 1.sp
                )
                Text(
                    text = "${apps.size} registered • ${processes.count { it.state == "running" }} active",
                    style = MaterialTheme.typography.bodySmall,
                    color = TextSecondary
                )
            }
            Button(
                onClick = { showAddDialog = true },
                colors = ButtonDefaults.buttonColors(containerColor = AccentCyan, contentColor = BgDark),
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier
                    .heightIn(min = 48.dp)
                    .testTag("add_windows_app_button")
            ) {
                Icon(Icons.Filled.Add, contentDescription = "Add Windows application", modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(6.dp))
                Text("Add EXE / MSI", fontWeight = FontWeight.Bold)
            }
        }

        if (apps.isEmpty()) {
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
                        .padding(20.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Icon(Icons.Filled.Apps, contentDescription = null, tint = TextMuted, modifier = Modifier.size(32.dp))
                    Text("No Windows Applications Added", style = MaterialTheme.typography.titleMedium, color = TextPrimary)
                    Text(
                        text = "Register an existing .exe or .msi installer, assign an isolated Wine prefix, choose a graphics backend, and launch through Box64 + Wine 11.0.",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                }
            }
        } else {
            apps.forEach { app ->
                val activeProc = processes.firstOrNull { it.app_id == app.app_id && it.state == "running" }
                WindowsAppCard(
                    app = app,
                    activeProcess = activeProc,
                    onLaunch = { onLaunchApp(app) },
                    onStop = { force -> onStopApp(app.app_id, force) },
                    onRemove = { onRemoveApp(app.app_id) }
                )
            }
        }
    }
}

@Composable
private fun WindowsAppCard(
    app: WindowsAppProfileData,
    activeProcess: TrackedWindowsProcessData?,
    onLaunch: () -> Unit,
    onStop: (Boolean) -> Unit,
    onRemove: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, if (activeProcess != null) AccentCyan.copy(alpha = 0.55f) else CardBorder, RoundedCornerShape(14.dp))
            .testTag("windows_app_card_${app.app_id}"),
        colors = CardDefaults.cardColors(containerColor = SurfaceDark),
        shape = RoundedCornerShape(14.dp)
    ) {
        Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(app.name, style = MaterialTheme.typography.titleMedium, color = TextPrimary, fontWeight = FontWeight.SemiBold)
                    Text(
                        text = app.executable_path,
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                }
                StatusBadgeChip(
                    label = if (activeProcess != null) "RUNNING" else app.graphics_backend.uppercase(),
                    color = if (activeProcess != null) StatusGreen else AccentCyan
                )
            }

            Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Prefix: ${app.prefix_id}", style = MaterialTheme.typography.labelSmall, color = TextMuted)
                Text("Wine ${app.wine_version}", style = MaterialTheme.typography.labelSmall, color = TextMuted)
                Text("${app.resolution_width}x${app.resolution_height}", style = MaterialTheme.typography.labelSmall, color = TextMuted)
            }

            if (app.compatibility_notes.isNotBlank()) {
                Text(app.compatibility_notes, style = MaterialTheme.typography.bodySmall, color = StarSilver)
            }

            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                Button(
                    onClick = onLaunch,
                    colors = ButtonDefaults.buttonColors(containerColor = AccentPurple, contentColor = TextPrimary),
                    shape = RoundedCornerShape(10.dp),
                    modifier = Modifier
                        .heightIn(min = 48.dp)
                        .testTag("launch_win_app_${app.app_id}")
                ) {
                    Icon(Icons.Filled.PlayArrow, contentDescription = "Launch ${app.name}", modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(4.dp))
                    Text("Launch")
                }
                if (activeProcess != null) {
                    OutlinedButton(
                        onClick = { onStop(false) },
                        border = BorderStroke(1.dp, StatusAmber),
                        shape = RoundedCornerShape(10.dp),
                        modifier = Modifier.heightIn(min = 48.dp)
                    ) {
                        Icon(Icons.Filled.Stop, contentDescription = "Stop ${app.name}", tint = StatusAmber, modifier = Modifier.size(18.dp))
                        Spacer(Modifier.width(4.dp))
                        Text("Stop", color = StatusAmber)
                    }
                }
                Spacer(Modifier.weight(1f))
                IconButton(
                    onClick = onRemove,
                    modifier = Modifier
                        .size(48.dp)
                        .testTag("remove_win_app_${app.app_id}")
                ) {
                    Icon(Icons.Filled.DeleteOutline, contentDescription = "Remove ${app.name}", tint = StatusRed)
                }
            }
        }
    }
}

@Composable
private fun AddWindowsAppDialog(
    selectedEnvId: String,
    prefixes: List<WinePrefixMetadataData>,
    onDismiss: () -> Unit,
    onConfirm: (WindowsAppProfileData) -> Unit
) {
    var name by remember { mutableStateOf("") }
    var exePath by remember { mutableStateOf("C:\\windows\\system32\\notepad.exe") }
    var argsText by remember { mutableStateOf("") }
    var prefixId by remember { mutableStateOf(prefixes.firstOrNull()?.id ?: "default-prefix") }
    var graphicsBackend by remember { mutableStateOf("software") }
    var resolution by remember { mutableStateOf("1280x720") }
    var notes by remember { mutableStateOf("") }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = SurfaceDark,
        title = { Text("Add Windows Application or Installer", color = TextPrimary) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    label = { Text("Application Name", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("win_app_name_input")
                )
                OutlinedTextField(
                    value = exePath,
                    onValueChange = { exePath = it },
                    label = { Text("EXE or MSI Path (Windows or Guest Linux)", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("win_app_path_input")
                )
                OutlinedTextField(
                    value = prefixId,
                    onValueChange = { prefixId = it },
                    label = { Text("Isolated Wine Prefix ID", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
                OutlinedTextField(
                    value = argsText,
                    onValueChange = { argsText = it },
                    label = { Text("Launch Arguments (optional)", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf("software", "turnip", "virgl").forEach { mode ->
                        FilterChip(
                            selected = graphicsBackend == mode,
                            onClick = { graphicsBackend = mode },
                            label = { Text(mode.uppercase(), style = MaterialTheme.typography.labelSmall) }
                        )
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf("1280x720", "1600x900", "1920x1080").forEach { res ->
                        FilterChip(
                            selected = resolution == res,
                            onClick = { resolution = res },
                            label = { Text(res, style = MaterialTheme.typography.labelSmall) }
                        )
                    }
                }
                OutlinedTextField(
                    value = notes,
                    onValueChange = { notes = it },
                    label = { Text("Compatibility Notes (optional)", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth()
                )
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    val cleanName = name.trim().ifEmpty {
                        exePath.substringAfterLast('\\').substringAfterLast('/').ifBlank { "Windows App" }
                    }
                    val appId = cleanName.lowercase().replace(Regex("[^a-z0-9_-]"), "-").take(40).ifEmpty { "win-app" }
                    val dims = resolution.split("x")
                    val w = dims.getOrNull(0)?.toIntOrNull() ?: 1280
                    val h = dims.getOrNull(1)?.toIntOrNull() ?: 720
                    onConfirm(
                        WindowsAppProfileData(
                            app_id = appId,
                            name = cleanName,
                            executable_path = exePath.trim(),
                            install_dir = exePath.trim().substringBeforeLast('\\', "").ifEmpty {
                                exePath.trim().substringBeforeLast('/', "")
                            },
                            prefix_id = prefixId.trim().ifEmpty { "default-prefix" },
                            environment_id = selectedEnvId,
                            wine_version = "11.0",
                            graphics_backend = graphicsBackend,
                            resolution_width = w,
                            resolution_height = h,
                            launch_args = argsText.trim().split(" ").filter { it.isNotEmpty() },
                            compatibility_notes = notes.trim()
                        )
                    )
                },
                colors = ButtonDefaults.buttonColors(containerColor = AccentCyan, contentColor = BgDark),
                modifier = Modifier.testTag("confirm_add_win_app_button")
            ) {
                Text("Save Profile", fontWeight = FontWeight.Bold)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel", color = TextSecondary) }
        }
    )
}

@Composable
internal fun StatusBadgeChip(label: String, color: androidx.compose.ui.graphics.Color) {
    Surface(
        color = color.copy(alpha = 0.16f),
        shape = RoundedCornerShape(999.dp),
        border = BorderStroke(1.dp, color.copy(alpha = 0.5f))
    ) {
        Text(
            text = label,
            color = color,
            style = MaterialTheme.typography.labelSmall,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp)
        )
    }
}
