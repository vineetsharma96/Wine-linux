package com.androidpc.environment.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.DeleteOutline
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Save
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.*
import com.androidpc.environment.ui.theme.*

@Composable
internal fun WineAndPrefixesTabContent(
    selectedEnvId: String,
    wineStatus: WineInstallationStatusData,
    prefixes: List<WinePrefixMetadataData>,
    processes: List<TrackedWindowsProcessData>,
    onCreatePrefix: (id: String, name: String, winVer: String, gfx: String) -> Unit,
    onPrefixAction: (action: String, prefixId: String) -> Unit,
    onStopProcess: (appId: String, force: Boolean) -> Unit
) {
    var showCreateDialog by remember { mutableStateOf(false) }

    if (showCreateDialog) {
        var pfxId by remember { mutableStateOf("prefix-${prefixes.size + 1}") }
        var pfxName by remember { mutableStateOf("Windows Container ${prefixes.size + 1}") }
        var winVer by remember { mutableStateOf("win10") }
        var gfx by remember { mutableStateOf("software") }

        AlertDialog(
            onDismissRequest = { showCreateDialog = false },
            containerColor = SurfaceDark,
            title = { Text("Create Isolated Wine Prefix", color = TextPrimary) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    OutlinedTextField(
                        value = pfxId,
                        onValueChange = { pfxId = it },
                        label = { Text("Prefix ID (alphanumeric, -, _)", color = TextSecondary) },
                        singleLine = true
                    )
                    OutlinedTextField(
                        value = pfxName,
                        onValueChange = { pfxName = it },
                        label = { Text("Display Name", color = TextSecondary) },
                        singleLine = true
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("win10", "win11", "win7").forEach { v ->
                            FilterChip(
                                selected = winVer == v,
                                onClick = { winVer = v },
                                label = { Text(v.uppercase(), style = MaterialTheme.typography.labelSmall) }
                            )
                        }
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("software", "turnip", "virgl").forEach { b ->
                            FilterChip(
                                selected = gfx == b,
                                onClick = { gfx = b },
                                label = { Text(b.uppercase(), style = MaterialTheme.typography.labelSmall) }
                            )
                        }
                    }
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        onCreatePrefix(pfxId, pfxName, winVer, gfx)
                        showCreateDialog = false
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = AccentCyan, contentColor = BgDark)
                ) {
                    Text("Create", fontWeight = FontWeight.Bold)
                }
            },
            dismissButton = {
                TextButton(onClick = { showCreateDialog = false }) { Text("Cancel", color = TextSecondary) }
            }
        )
    }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        // Wine Runtime Card
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
                    Column(modifier = Modifier.weight(1f)) {
                        Text("Wine ${wineStatus.pinned_version} (WoW64)", style = MaterialTheme.typography.titleMedium, color = TextPrimary, fontWeight = FontWeight.SemiBold)
                        Text(wineStatus.architecture_strategy, style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                    }
                    StatusBadgeChip(
                        label = if (wineStatus.installed) "INSTALLED" else "NOT STAGED",
                        color = if (wineStatus.installed) StatusGreen else StatusAmber
                    )
                }
                Text(
                    text = "License: ${wineStatus.license} • Box86 required: ${if (wineStatus.box86_required) "Yes" else "No (pure-64-bit WoW64)"}",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextMuted
                )
                wineStatus.diagnostics.forEach { line ->
                    Text("• $line", style = MaterialTheme.typography.bodySmall, color = StarSilver)
                }
            }
        }

        // Isolated Prefixes Header
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text("ISOLATED WINE PREFIXES (${prefixes.size})", style = MaterialTheme.typography.labelSmall, color = TextMuted, letterSpacing = 1.sp)
            Button(
                onClick = { showCreateDialog = true },
                colors = ButtonDefaults.buttonColors(containerColor = AccentCyan, contentColor = BgDark),
                shape = RoundedCornerShape(10.dp),
                modifier = Modifier.heightIn(min = 48.dp).testTag("create_wine_prefix_button")
            ) {
                Icon(Icons.Filled.Add, contentDescription = "Create Wine prefix", modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(4.dp))
                Text("New Prefix", fontWeight = FontWeight.Bold)
            }
        }

        if (prefixes.isEmpty()) {
            Card(
                modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(14.dp)
            ) {
                Text(
                    text = "No isolated Wine prefixes created for '$selectedEnvId'. Create separate prefixes to prevent Windows applications from corrupting each other's registry or DLL overrides.",
                    style = MaterialTheme.typography.bodySmall,
                    color = TextSecondary,
                    modifier = Modifier.padding(16.dp)
                )
            }
        } else {
            prefixes.forEach { pfx ->
                Card(
                    modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(pfx.display_name, style = MaterialTheme.typography.titleSmall, color = TextPrimary, fontWeight = FontWeight.SemiBold)
                                Text("${pfx.prefix_path} • ${pfx.arch} • ${pfx.windows_version.uppercase()}", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                            }
                            StatusBadgeChip(pfx.graphics_backend.uppercase(), AccentPurple)
                        }
                        Text(
                            text = "Apps: ${pfx.installed_apps.size} • Deps: ${pfx.dependencies.size} • Env: ${pfx.environment_id.ifBlank { selectedEnvId }}",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            OutlinedButton(
                                onClick = { onPrefixAction("clone", pfx.id) },
                                border = BorderStroke(1.dp, CardBorder),
                                modifier = Modifier.heightIn(min = 48.dp)
                            ) {
                                Icon(Icons.Filled.ContentCopy, contentDescription = "Clone prefix", tint = AccentCyan, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("Clone", color = TextPrimary, style = MaterialTheme.typography.labelSmall)
                            }
                            OutlinedButton(
                                onClick = { onPrefixAction("backup", pfx.id) },
                                border = BorderStroke(1.dp, CardBorder),
                                modifier = Modifier.heightIn(min = 48.dp)
                            ) {
                                Icon(Icons.Filled.Save, contentDescription = "Backup prefix", tint = AccentCyan, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("Backup", color = TextPrimary, style = MaterialTheme.typography.labelSmall)
                            }
                            OutlinedButton(
                                onClick = { onPrefixAction("repair", pfx.id) },
                                border = BorderStroke(1.dp, CardBorder),
                                modifier = Modifier.heightIn(min = 48.dp)
                            ) {
                                Icon(Icons.Filled.Build, contentDescription = "Repair prefix", tint = StatusAmber, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("Repair", color = TextPrimary, style = MaterialTheme.typography.labelSmall)
                            }
                            Spacer(Modifier.weight(1f))
                            IconButton(
                                onClick = { onPrefixAction("delete", pfx.id) },
                                modifier = Modifier.size(48.dp)
                            ) {
                                Icon(Icons.Filled.DeleteOutline, contentDescription = "Delete prefix", tint = StatusRed)
                            }
                        }
                    }
                }
            }
        }

        // Supervised Windows Processes
        if (processes.isNotEmpty()) {
            Text("SUPERVISED WINDOWS PROCESSES", style = MaterialTheme.typography.labelSmall, color = TextMuted, letterSpacing = 1.sp)
            processes.forEach { proc ->
                Card(
                    modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceElevated),
                    shape = RoundedCornerShape(12.dp)
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(12.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("${proc.app_name} (${proc.state})", style = MaterialTheme.typography.titleSmall, color = TextPrimary)
                            Text(
                                text = "Prefix: ${proc.prefix_id} • PID: ${proc.wine_pid ?: "staged"} • RSS: ${proc.memory_rss_bytes?.let { "${it / (1024 * 1024)} MiB" } ?: "N/A"}",
                                style = MaterialTheme.typography.labelSmall,
                                color = TextSecondary
                            )
                        }
                        if (proc.state == "running") {
                            IconButton(onClick = { onStopProcess(proc.app_id, false) }, modifier = Modifier.size(48.dp)) {
                                Icon(Icons.Filled.Stop, contentDescription = "Terminate ${proc.app_name}", tint = StatusRed)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
internal fun GraphicsAndDxTabContent(
    graphics: GraphicsStackOverviewData,
    onUpdateConfig: (GraphicsBackendConfigData) -> Unit
) {
    val cfg = graphics.active_config

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        // Active Backend & Fallback Card
        Card(
            modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
            colors = CardDefaults.cardColors(containerColor = SurfaceDark),
            shape = RoundedCornerShape(14.dp)
        ) {
            Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text("GRAPHICS BACKEND SELECTION", style = MaterialTheme.typography.labelSmall, color = TextMuted, letterSpacing = 1.sp)
                Text(
                    text = "Effective: ${graphics.effective_backend.uppercase()} • DirectX Path: ${graphics.dx_translation.effective_choice.uppercase()}",
                    style = MaterialTheme.typography.titleSmall,
                    color = AccentCyan
                )
                graphics.fallback_reason?.let {
                    Text(it, style = MaterialTheme.typography.bodySmall, color = StatusAmber)
                }

                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    listOf("turnip" to "Turnip", "virgl" to "VirGL", "software" to "Software").forEach { (key, label) ->
                        FilterChip(
                            selected = cfg.preferred_backend == key,
                            onClick = { onUpdateConfig(cfg.copy(preferred_backend = key)) },
                            label = { Text(label) },
                            modifier = Modifier.testTag("gfx_select_$key")
                        )
                    }
                }

                HorizontalDivider(color = CardBorder)

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("DXVK v${graphics.dx_translation.dxvk_pinned_version} (D3D 8/9/10/11)", style = MaterialTheme.typography.bodyMedium, color = TextPrimary)
                        Text(
                            text = graphics.dx_translation.dxvk_fallback_reason ?: "Eligible on operational Vulkan backend",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                    }
                    Switch(
                        checked = cfg.dxvk_enabled,
                        onCheckedChange = { onUpdateConfig(cfg.copy(dxvk_enabled = it)) }
                    )
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("VKD3D-Proton v${graphics.dx_translation.vkd3d_pinned_version} (D3D 12)", style = MaterialTheme.typography.bodyMedium, color = TextPrimary)
                        Text(
                            text = graphics.dx_translation.vkd3d_fallback_reason ?: "All required Vulkan 1.3 extensions verified",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                    }
                    Switch(
                        checked = cfg.vkd3d_proton_enabled,
                        onCheckedChange = { onUpdateConfig(cfg.copy(vkd3d_proton_enabled = it)) }
                    )
                }
            }
        }

        // Honest 5-Stage Backend Capability Cards
        Text("BACKEND READINESS & CAPABILITIES", style = MaterialTheme.typography.labelSmall, color = TextMuted, letterSpacing = 1.sp)
        graphics.backends.forEach { b ->
            val stageColor = when (b.stage) {
                "operational", "active_in_app" -> StatusGreen
                "detected", "initialized" -> AccentCyan
                "installed" -> StatusAmber
                else -> TextMuted
            }
            Card(
                modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(b.display_name, style = MaterialTheme.typography.titleSmall, color = TextPrimary, modifier = Modifier.weight(1f))
                        StatusBadgeChip(b.stage.replace('_', ' ').uppercase(), stageColor)
                    }
                    Text(
                        text = "v${b.pinned_version} (${b.license}) • OpenGL: ${if (b.supports_opengl) "Yes" else "No"} • Vulkan: ${if (b.supports_vulkan) "Yes" else "No"} • DXVK: ${if (b.dxvk_compatible) "Ready" else "Gated"}",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextSecondary
                    )
                    b.detected_gpu?.let {
                        Text(it, style = MaterialTheme.typography.bodySmall, color = StarSilver)
                    }
                    b.warnings.forEach { w ->
                        Text("! $w", style = MaterialTheme.typography.bodySmall, color = StatusAmber)
                    }
                }
            }
        }
    }
}

@Composable
internal fun DependenciesAndDiagnosticsTabContent(
    overview: WindowsCompatibilityOverviewData,
    dependencies: List<DependencyProfileData>,
    diagnosis: PrefixDependencyDiagnosisData?,
    onRunDiagnosis: () -> Unit,
    onOpenBox64: () -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        // Box64 Quick Card
        Card(
            modifier = Modifier.fillMaxWidth().border(1.dp, AccentPurple.copy(alpha = 0.5f), RoundedCornerShape(14.dp)),
            colors = CardDefaults.cardColors(containerColor = SurfaceElevated),
            shape = RoundedCornerShape(14.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(14.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(
                    modifier = Modifier.weight(1f),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(Icons.Filled.Memory, contentDescription = null, tint = AccentCyan)
                    Column {
                        Text("Box64 v${overview.box64.pinned_version} Settings & Suite", style = MaterialTheme.typography.titleSmall, color = TextPrimary)
                        Text(
                            text = if (overview.box64.installed) "Staged • Dynarec/Interpreter controls" else "Not staged yet • Configure ELF & Dynarec",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                    }
                }
                TextButton(onClick = onOpenBox64, modifier = Modifier.heightIn(min = 48.dp)) {
                    Text("Open", color = AccentCyan, fontWeight = FontWeight.Bold)
                }
            }
        }

        // Runtime Errors & Diagnostics
        if (overview.runtime_errors.isNotEmpty()) {
            Card(
                modifier = Modifier.fillMaxWidth().border(1.dp, StatusAmber.copy(alpha = 0.5f), RoundedCornerShape(14.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text("RUNTIME DIAGNOSTICS & BLOCKERS", style = MaterialTheme.typography.labelSmall, color = StatusAmber, letterSpacing = 1.sp)
                    overview.runtime_errors.forEach { err ->
                        Text("• $err", style = MaterialTheme.typography.bodySmall, color = TextPrimary)
                    }
                }
            }
        }

        // Dependency Scan & Catalog
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text("WINDOWS DEPENDENCY PROFILES", style = MaterialTheme.typography.labelSmall, color = TextMuted, letterSpacing = 1.sp)
            OutlinedButton(
                onClick = onRunDiagnosis,
                border = BorderStroke(1.dp, AccentCyan),
                modifier = Modifier.heightIn(min = 48.dp).testTag("scan_prefix_dlls_button")
            ) {
                Text("Scan Missing DLLs", color = AccentCyan, style = MaterialTheme.typography.labelSmall)
            }
        }

        diagnosis?.let { diag ->
            Card(
                modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceElevated),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Prefix '${diag.prefix_id}' Scan", style = MaterialTheme.typography.titleSmall, color = TextPrimary)
                    Text(
                        "Present DLLs: ${diag.present_dlls.size} • Missing: ${diag.missing_dlls.size} • Conflicts: ${diag.conflicts.size}",
                        style = MaterialTheme.typography.labelSmall,
                        color = AccentCyan
                    )
                    diag.notes.forEach { note ->
                        Text("• $note", style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                    }
                }
            }
        }

        dependencies.forEach { dep ->
            Card(
                modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(dep.display_name, style = MaterialTheme.typography.titleSmall, color = TextPrimary, modifier = Modifier.weight(1f))
                        StatusBadgeChip(dep.version, AccentCyan)
                    }
                    Text("${dep.license} • ${dep.mechanism.replace('_', ' ')}", style = MaterialTheme.typography.labelSmall, color = TextMuted)
                    Text(dep.recovery_hint, style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                }
            }
        }
    }
}
