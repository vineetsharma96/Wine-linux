package com.androidpc.environment.ui.screens

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.ErrorOutline
import androidx.compose.material.icons.filled.ExpandLess
import androidx.compose.material.icons.filled.ExpandMore
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Schedule
import androidx.compose.material.icons.filled.Science
import androidx.compose.material.icons.filled.VerifiedUser
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.*
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CompatibilityScreen(
    environments: List<EnvironmentConfigData>,
    selectedEnvId: String,
    onSelectEnv: (String) -> Unit,
    onBack: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var status by remember { mutableStateOf<Box64InstallationStatusData?>(null) }
    var config by remember { mutableStateOf(Box64EnvironmentConfigData()) }
    var suiteReport by remember { mutableStateOf<CompatibilitySuiteReportData?>(null) }
    var history by remember { mutableStateOf<List<Box64ExecutionRecordData>>(emptyList()) }
    var box64Logs by remember { mutableStateOf<List<LogEntryData>>(emptyList()) }
    var showAdvanced by remember { mutableStateOf(false) }

    var binaryInput by remember { mutableStateOf("/usr/bin/env") }
    var argsInput by remember { mutableStateOf("true") }
    var isRunningBinary by remember { mutableStateOf(false) }
    var lastOutcome by remember { mutableStateOf<Box64ExecutionOutcomeData?>(null) }
    var actionBanner by remember { mutableStateOf<String?>(null) }

    val refreshAll: () -> Unit = {
        scope.launch {
            status = NativeBridge.getBox64Status(context, selectedEnvId)
            config = NativeBridge.getBox64Config(context, selectedEnvId)
            suiteReport = NativeBridge.runBox64Suite(context, selectedEnvId)
            history = NativeBridge.getBox64History(20)
            NativeBridge.getRecentLogs(60).onSuccess { entries ->
                box64Logs = entries.filter {
                    it.target.contains("box64", ignoreCase = true) ||
                        it.message.contains("box64", ignoreCase = true) ||
                        it.message.contains("x86", ignoreCase = true)
                }.reversed().take(20)
            }
        }
    }

    LaunchedEffect(selectedEnvId) {
        refreshAll()
    }

    val saveConfig: (Box64EnvironmentConfigData) -> Unit = { updated ->
        config = updated
        scope.launch {
            NativeBridge.updateBox64Config(context, selectedEnvId, updated)
                .onSuccess { saved ->
                    config = saved
                    actionBanner = "Box64 configuration saved for ${selectedEnvId.ifEmpty { "default environment" }}"
                }
                .onFailure { err ->
                    actionBanner = "Failed to save Box64 settings: ${err.message}"
                }
        }
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
                        onClick = onBack,
                        modifier = Modifier.testTag("box64_back_button")
                    ) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Back to Containers",
                            tint = AccentCyan
                        )
                    }
                },
                title = {
                    Column {
                        Text(
                            text = "Box64 Compatibility",
                            style = MaterialTheme.typography.titleLarge,
                            color = TextPrimary
                        )
                        Text(
                            text = "x86-64 Linux Translation Layer",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                    }
                },
                actions = {
                    IconButton(
                        onClick = refreshAll,
                        modifier = Modifier.testTag("box64_refresh_button")
                    ) {
                        Icon(
                            imageVector = Icons.Filled.Refresh,
                            contentDescription = "Refresh Box64 Status",
                            tint = AccentCyan
                        )
                    }
                }
            )
        }
    ) { innerPadding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(horizontal = 16.dp),
            contentPadding = PaddingValues(vertical = 16.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            if (environments.isNotEmpty()) {
                item {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        environments.forEach { env ->
                            val selected = env.id == selectedEnvId
                            FilterChip(
                                selected = selected,
                                onClick = { onSelectEnv(env.id) },
                                label = {
                                    Text(
                                        text = env.display_name ?: "${env.distribution.replaceFirstChar(Char::uppercase)} ${env.version}",
                                        style = MaterialTheme.typography.labelMedium
                                    )
                                },
                                colors = FilterChipDefaults.filterChipColors(
                                    selectedContainerColor = AccentPurple.copy(alpha = 0.25f),
                                    selectedLabelColor = AccentCyan,
                                    containerColor = SurfaceDark,
                                    labelColor = TextSecondary
                                ),
                                border = FilterChipDefaults.filterChipBorder(
                                    enabled = true,
                                    selected = selected,
                                    borderColor = CardBorder,
                                    selectedBorderColor = AccentCyan
                                )
                            )
                        }
                    }
                }
            }

            actionBanner?.let { banner ->
                item {
                    Surface(
                        color = SurfaceContainer,
                        shape = RoundedCornerShape(10.dp),
                        border = BorderStroke(1.dp, AccentCyan.copy(alpha = 0.4f)),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text(
                            text = banner,
                            style = MaterialTheme.typography.bodySmall,
                            color = AccentCyan,
                            modifier = Modifier.padding(12.dp)
                        )
                    }
                }
            }

            // 1. Box64 Installation & ABI Status Card
            item {
                val st = status
                val installed = st?.installed == true
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, if (installed) StatusGreen.copy(alpha = 0.5f) else CardBorder, RoundedCornerShape(14.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Icon(
                                    imageVector = Icons.Filled.Memory,
                                    contentDescription = null,
                                    tint = AccentCyan
                                )
                                Text(
                                    text = "BOX64 RUNTIME STATUS",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextMuted,
                                    letterSpacing = 1.sp
                                )
                            }
                            StatusPill(
                                text = if (installed) "INSTALLED" else "PENDING BINARY",
                                color = if (installed) StatusGreen else StatusAmber
                            )
                        }

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            MetricItem("Version", st?.detected_version ?: "v${st?.pinned_version ?: "0.4.4"}")
                            MetricItem("License", st?.license ?: "MIT")
                            MetricItem("Host ABI", st?.host_arch ?: "aarch64")
                        }

                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(6.dp)
                        ) {
                            Icon(
                                imageVector = Icons.Filled.VerifiedUser,
                                contentDescription = null,
                                tint = if (st?.abi_isolation_verified != false) StatusGreen else StatusRed,
                                modifier = Modifier.size(16.dp)
                            )
                            Text(
                                text = if (st?.abi_isolation_verified != false) {
                                    "Guest Linux glibc isolated from Android Bionic libraries"
                                } else {
                                    "ABI Warning: Android Bionic linkage detected — rebuild with aarch64-linux-gnu"
                                },
                                style = MaterialTheme.typography.bodySmall,
                                color = TextSecondary
                            )
                        }

                        Text(
                            text = st?.status_summary?.ifEmpty { "Checking Box64 installation..." } ?: "Checking Box64 installation...",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextMuted
                        )
                    }
                }
            }

            // 2. Simple Enable/Disable & Mode Controls (Complex Settings Hidden by Default)
            item {
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = "Enable x86-64 Compatibility",
                                    style = MaterialTheme.typography.titleMedium,
                                    color = TextPrimary
                                )
                                Text(
                                    text = "ARM64 binaries always execute natively; x86-64 binaries route through Box64.",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = TextSecondary
                                )
                            }
                            Spacer(modifier = Modifier.width(12.dp))
                            Switch(
                                checked = config.enabled,
                                onCheckedChange = { saveConfig(config.copy(enabled = it)) },
                                modifier = Modifier.testTag("box64_enable_switch")
                            )
                        }

                        HorizontalDivider(color = CardBorder)

                        Text(
                            text = "EXECUTION ENGINE",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted,
                            letterSpacing = 1.sp
                        )

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(10.dp)
                        ) {
                            ModeChoiceButton(
                                label = "ARM64 Dynarec",
                                subtitle = "Conservative default",
                                selected = config.dynarec_enabled,
                                enabled = config.enabled,
                                modifier = Modifier.weight(1f).testTag("box64_dynarec_button"),
                                onClick = { saveConfig(config.copy(dynarec_enabled = true)) }
                            )
                            ModeChoiceButton(
                                label = "Interpreter",
                                subtitle = "Diagnostic fallback",
                                selected = !config.dynarec_enabled,
                                enabled = config.enabled,
                                modifier = Modifier.weight(1f).testTag("box64_interpreter_button"),
                                onClick = { saveConfig(config.copy(dynarec_enabled = false)) }
                            )
                        }

                        // Expandable advanced diagnostics settings (collapsed by default)
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable { showAdvanced = !showAdvanced }
                                .padding(vertical = 4.dp),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(
                                text = "Advanced Runtime Diagnostics & Flags",
                                style = MaterialTheme.typography.labelMedium,
                                color = AccentCyan
                            )
                            Icon(
                                imageVector = if (showAdvanced) Icons.Filled.ExpandLess else Icons.Filled.ExpandMore,
                                contentDescription = "Toggle Advanced Box64 Settings",
                                tint = AccentCyan
                            )
                        }

                        AnimatedVisibility(visible = showAdvanced) {
                            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                                Text(
                                    text = "Conservative defaults keep SAFEFLAGS=1, STRONGMEM=1, BIGBLOCK=1 to avoid instability.",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = TextMuted
                                )
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    MetricItem("SAFEFLAGS", "${config.dynarec_safeflags}")
                                    MetricItem("STRONGMEM", "${config.dynarec_strongmem}")
                                    MetricItem("BIGBLOCK", "${config.dynarec_bigblock}")
                                    MetricItem("LOG", config.log_level.uppercase())
                                }
                                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                    listOf("none", "errors", "info", "verbose").forEach { level ->
                                        val selected = config.log_level.equals(level, ignoreCase = true)
                                        OutlinedButton(
                                            onClick = { saveConfig(config.copy(log_level = level)) },
                                            colors = ButtonDefaults.outlinedButtonColors(
                                                containerColor = if (selected) AccentCyan.copy(alpha = 0.18f) else SurfaceContainer
                                            ),
                                            border = BorderStroke(1.dp, if (selected) AccentCyan else CardBorder),
                                            contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                                            shape = RoundedCornerShape(8.dp)
                                        ) {
                                            Text(
                                                text = level.uppercase(),
                                                style = MaterialTheme.typography.labelSmall,
                                                color = if (selected) AccentCyan else TextSecondary
                                            )
                                        }
                                    }
                                }
                                Text(
                                    text = "Guest x86-64 library paths: ${config.library_search_paths.joinToString(":")}",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextSecondary
                                )
                            }
                        }
                    }
                }
            }

            // 3. Binary Architecture Inspector & Execution Runner
            item {
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Text(
                            text = "ELF ROUTER & BINARY TEST RUNNER",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted,
                            letterSpacing = 1.sp
                        )
                        OutlinedTextField(
                            value = binaryInput,
                            onValueChange = { binaryInput = it },
                            label = { Text("Guest Binary Path") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth().testTag("box64_binary_input")
                        )
                        OutlinedTextField(
                            value = argsInput,
                            onValueChange = { argsInput = it },
                            label = { Text("Arguments (space-separated)") },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth()
                        )
                        Button(
                            onClick = {
                                if (selectedEnvId.isEmpty()) {
                                    actionBanner = "Create or select a Linux environment first."
                                    return@Button
                                }
                                isRunningBinary = true
                                scope.launch {
                                    val parsedArgs = argsInput.trim().split(" ").filter { it.isNotEmpty() }
                                    NativeBridge.executeCompatibleBinary(selectedEnvId, binaryInput.trim(), parsedArgs)
                                        .onSuccess { outcome ->
                                            lastOutcome = outcome
                                            actionBanner = null
                                            history = NativeBridge.getBox64History(20)
                                        }
                                        .onFailure { err ->
                                            actionBanner = err.message ?: "Execution failed"
                                        }
                                    isRunningBinary = false
                                }
                            },
                            enabled = !isRunningBinary,
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(48.dp)
                                .testTag("box64_run_binary_button"),
                            colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                            shape = RoundedCornerShape(10.dp)
                        ) {
                            Icon(Icons.Filled.PlayArrow, contentDescription = null, tint = BgDark)
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = if (isRunningBinary) "Executing..." else "Detect ELF & Execute",
                                color = BgDark,
                                fontWeight = FontWeight.Bold
                            )
                        }

                        lastOutcome?.let { out ->
                            Surface(
                                color = SurfaceContainer,
                                shape = RoundedCornerShape(10.dp),
                                border = BorderStroke(
                                    1.dp,
                                    if (out.success) StatusGreen.copy(alpha = 0.5f) else StatusRed.copy(alpha = 0.5f)
                                ),
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Column(
                                    modifier = Modifier.padding(12.dp),
                                    verticalArrangement = Arrangement.spacedBy(6.dp)
                                ) {
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween
                                    ) {
                                        Text(
                                            text = "${out.route.uppercase()} • ${out.architecture}",
                                            style = MaterialTheme.typography.labelSmall,
                                            color = AccentCyan
                                        )
                                        Text(
                                            text = "${out.duration_ms} ms",
                                            style = MaterialTheme.typography.labelSmall,
                                            color = TextMuted
                                        )
                                    }
                                    out.error_explanation?.let { explanation ->
                                        Text(
                                            text = explanation,
                                            style = MaterialTheme.typography.bodySmall,
                                            color = StatusRed
                                        )
                                    }
                                    if (out.missing_libraries.isNotEmpty()) {
                                        Text(
                                            text = "Missing libraries: ${out.missing_libraries.joinToString(", ")}",
                                            style = MaterialTheme.typography.labelSmall,
                                            color = StatusAmber
                                        )
                                    }
                                    if (out.stdout.isNotBlank()) {
                                        Text(
                                            text = out.stdout.take(400),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = TextPrimary
                                        )
                                    }
                                    if (out.stderr.isNotBlank()) {
                                        Text(
                                            text = out.stderr.take(400),
                                            style = MaterialTheme.typography.labelSmall,
                                            color = StatusAmber
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 4. Compatibility Test Suite Results (Categories A-D)
            item {
                val report = suiteReport
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Column(
                        modifier = Modifier.padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Icon(Icons.Filled.Science, contentDescription = null, tint = AccentPurple)
                                Text(
                                    text = "COMPATIBILITY TEST SUITE",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextMuted,
                                    letterSpacing = 1.sp
                                )
                            }
                            OutlinedButton(
                                onClick = refreshAll,
                                shape = RoundedCornerShape(8.dp),
                                border = BorderStroke(1.dp, CardBorder),
                                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                                modifier = Modifier.testTag("box64_run_suite_button")
                            ) {
                                Text("Run Suite", color = AccentCyan, style = MaterialTheme.typography.labelMedium)
                            }
                        }

                        if (report != null) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(12.dp)
                            ) {
                                StatusPill("PASSED: ${report.passed}", StatusGreen)
                                StatusPill("DEVICE REQ: ${report.device_required}", StatusAmber)
                                StatusPill("FAILED: ${report.failed}", if (report.failed > 0) StatusRed else TextMuted)
                            }

                            report.cases.forEach { tc ->
                                val (icon, color) = when (tc.status) {
                                    "passed" -> Icons.Filled.CheckCircle to StatusGreen
                                    "failed" -> Icons.Filled.ErrorOutline to StatusRed
                                    else -> Icons.Filled.Schedule to StatusAmber
                                }
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .background(SurfaceContainer, RoundedCornerShape(10.dp))
                                        .padding(10.dp),
                                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                                    verticalAlignment = Alignment.Top
                                ) {
                                    Icon(icon, contentDescription = tc.status, tint = color, modifier = Modifier.size(18.dp))
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(tc.title, style = MaterialTheme.typography.bodyMedium, color = TextPrimary, fontWeight = FontWeight.Medium)
                                        Text(tc.detail, style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 5. Application Execution History
            item {
                Text(
                    text = "COMPATIBILITY EXECUTION HISTORY (${history.size})",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextMuted,
                    letterSpacing = 1.sp
                )
            }

            if (history.isEmpty()) {
                item {
                    Surface(
                        color = SurfaceDark,
                        shape = RoundedCornerShape(12.dp),
                        border = BorderStroke(1.dp, CardBorder),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text(
                            text = "No executions recorded yet. Run a guest binary above to inspect ELF routing, shared libraries, and exit diagnostics.",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary,
                            modifier = Modifier.padding(14.dp)
                        )
                    }
                }
            } else {
                items(history) { item ->
                    Card(
                        modifier = Modifier
                            .fillMaxWidth()
                            .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
                        colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                        shape = RoundedCornerShape(10.dp)
                    ) {
                        Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(item.binary_path, style = MaterialTheme.typography.titleMedium, color = TextPrimary)
                                StatusPill(
                                    text = item.route.uppercase(),
                                    color = if (item.success) StatusGreen else StatusRed
                                )
                            }
                            Text(
                                text = "${item.architecture} • ${item.duration_ms} ms • ${item.summary}",
                                style = MaterialTheme.typography.bodySmall,
                                color = TextSecondary
                            )
                        }
                    }
                }
            }

            // 6. Box64 Runtime Logs
            if (box64Logs.isNotEmpty()) {
                item {
                    Text(
                        text = "BOX64 RUNTIME LOGS (${box64Logs.size})",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextMuted,
                        letterSpacing = 1.sp
                    )
                }
                items(box64Logs) { entry ->
                    Surface(
                        color = SurfaceDark,
                        shape = RoundedCornerShape(8.dp),
                        border = BorderStroke(1.dp, CardBorder),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Column(modifier = Modifier.padding(10.dp)) {
                            Text(
                                text = "[${entry.level}] ${entry.target}",
                                style = MaterialTheme.typography.labelSmall,
                                color = if (entry.level == "ERROR") StatusRed else AccentCyan
                            )
                            Text(
                                text = entry.message,
                                style = MaterialTheme.typography.labelSmall,
                                color = TextPrimary
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ModeChoiceButton(
    label: String,
    subtitle: String,
    selected: Boolean,
    enabled: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit
) {
    Surface(
        modifier = modifier
            .border(
                width = 1.dp,
                color = if (selected && enabled) AccentCyan else CardBorder,
                shape = RoundedCornerShape(10.dp)
            )
            .clickable(enabled = enabled, onClick = onClick),
        color = if (selected && enabled) AccentCyan.copy(alpha = 0.14f) else SurfaceContainer,
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelLarge,
                color = if (selected && enabled) AccentCyan else TextPrimary
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = TextMuted
            )
        }
    }
}

@Composable
fun StatusPill(text: String, color: Color) {
    Surface(
        color = color.copy(alpha = 0.15f),
        shape = RoundedCornerShape(6.dp),
        border = BorderStroke(1.dp, color.copy(alpha = 0.35f))
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            color = color,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 3.dp)
        )
    }
}
