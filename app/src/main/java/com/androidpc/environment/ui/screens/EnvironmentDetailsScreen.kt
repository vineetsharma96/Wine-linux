package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.EnvironmentConfigData
import com.androidpc.environment.bridge.EnvironmentMetricsData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.bridge.PackageOperationData
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private enum class PackageAction(val operation: String, val label: String, val requiresInput: Boolean, val changesPackages: Boolean) {
    Installed("list", "List installed", false, false),
    Search("search", "Search", true, false),
    Details("details", "Show details", true, false),
    Update("update", "Refresh index", false, false),
    Upgrade("upgrade", "Upgrade packages", false, true),
    Install("install", "Install packages", true, true),
    Remove("remove", "Remove packages", true, true),
    Repair("repair", "Configure pending packages", false, true),
}

@Composable
fun EnvironmentDetailsScreen(
    environment: EnvironmentConfigData,
    state: String?,
    usage: Long?,
    onBackups: () -> Unit,
    onBack: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    var action by remember(environment.id) { mutableStateOf(PackageAction.Installed) }
    var query by remember(environment.id) { mutableStateOf("") }
    var busy by remember(environment.id) { mutableStateOf(false) }
    var cancelling by remember(environment.id) { mutableStateOf(false) }
    var confirmation by remember(environment.id) { mutableStateOf(false) }
    var result by remember(environment.id) { mutableStateOf<PackageOperationData?>(null) }
    var error by remember(environment.id) { mutableStateOf<String?>(null) }
    var metrics by remember(environment.id) { mutableStateOf<EnvironmentMetricsData?>(null) }
    LaunchedEffect(environment.id) { metrics = NativeBridge.environmentMetrics(environment.id) }
    val run = {
        busy = true
        error = null
        result = null
        scope.launch {
            try {
                NativeBridge.runPackageOperation(environment.id, action.operation, query)
                    .onSuccess { result = it }
                    .onFailure { error = it.message ?: "Couldn't run the package operation" }
            } finally {
                busy = false
                cancelling = false
            }
        }
    }
    LaunchedEffect(environment.id, busy) {
        while (busy) {
            val progress = NativeBridge.packageOperationProgress(environment.id)
            if (busy && progress != null) result = progress
            delay(400)
        }
    }

    if (confirmation) {
        AlertDialog(
            onDismissRequest = { confirmation = false },
            title = { Text(action.label) },
            text = { Text("Apply this change inside ${environment.display_name ?: environment.distribution}?" +
                if (action.requiresInput) "\n\n${query.trim()}" else "") },
            confirmButton = { TextButton(onClick = { confirmation = false; run() }) { Text("Apply") } },
            dismissButton = { TextButton(onClick = { confirmation = false }) { Text("Cancel") } }
        )
    }

    Column(
        Modifier.fillMaxSize().background(BgDark).verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        TextButton(onClick = onBack, enabled = !busy) { Text("← Back", color = AccentCyan) }
        Text(environment.display_name ?: environment.distribution.replaceFirstChar(Char::uppercase),
            style = MaterialTheme.typography.headlineSmall, color = TextPrimary)
        Text("${environment.distribution} ${environment.version} · ${environment.architecture} · " +
            if (busy) "Updating" else state.orEmpty(), color = TextSecondary)
        usage?.let { Text("${it / (1024 * 1024)} MiB allocated", color = TextSecondary) }
        environment.runtime_version?.let { Text("PRoot $it", color = TextSecondary) }
        metrics?.let { value ->
            Text(
                "Measured: ${value.processes} supervised PRoot & PTY (${value.terminal_sessions} terminals); guest child processes excluded",
                color = TextSecondary
            )
            Text(
                "Resident memory: ${value.memory_rss_bytes?.let { "${it / (1024 * 1024)} MiB" } ?: "Unavailable"}",
                color = TextSecondary
            )
            Text(
                "CPU time: ${value.cpu_time_ms?.let { "$it ms" } ?: "Unavailable"}",
                color = TextSecondary
            )
            Text(
                "Active runtime: ${value.seconds_since_launch?.let { "${it}s since launch" } ?: "Unavailable (stopped)"}",
                color = TextSecondary
            )
        } ?: Text("Resource measurements unavailable", color = TextSecondary)
        TextButton(onClick = { scope.launch { metrics = NativeBridge.environmentMetrics(environment.id) } }) {
            Text("Refresh resources")
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = onBackups, enabled = !busy) { Text("Back up or clone") }
            OutlinedButton(
                onClick = {
                    busy = true; error = null; result = null
                    scope.launch {
                        val raw = NativeBridge.installDesktopProfile(environment.id, "minimal")
                        if (raw.contains("\"error\"")) error = raw else result = com.google.gson.Gson().fromJson(raw, PackageOperationData::class.java)
                        busy = false
                    }
                },
                enabled = !busy
            ) { Text("Install XFCE") }
        }
        HorizontalDivider()
        Text("Packages", style = MaterialTheme.typography.titleMedium, color = TextPrimary)
        Text("Stop the environment before using its package manager.", color = TextSecondary)
        PackageAction.entries.chunked(2).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { option ->
                    FilterChip(
                        selected = action == option,
                        onClick = { action = option; error = null; result = null },
                        label = { Text(option.label) },
                        enabled = !busy
                    )
                }
            }
        }
        if (action.requiresInput) {
            OutlinedTextField(
                value = query, onValueChange = { if (it.length <= 1024) query = it },
                label = { Text(if (action == PackageAction.Search) "Search term" else "Package names") },
                singleLine = true, enabled = !busy, modifier = Modifier.fillMaxWidth()
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                onClick = { if (action.changesPackages) confirmation = true else run() },
                enabled = !busy && (!action.requiresInput || query.isNotBlank())
            ) { Text(action.label) }
            if (busy) {
                TextButton(
                    onClick = { scope.launch { cancelling = NativeBridge.cancelPackageOperation(environment.id) } },
                    enabled = !cancelling
                ) { Text(if (cancelling) "Cancelling…" else "Cancel operation") }
            }
        }
        if (busy) LinearProgressIndicator(Modifier.fillMaxWidth())
        error?.let { Text(it, color = StatusRed) }
        result?.let { operation ->
            operation.exit_code?.let { code ->
                Text(
                    if (operation.cancelled) "Cancelled · exit $code" else "Exit $code",
                    color = if (code == 0 && !operation.cancelled) StatusGreen else StatusAmber
                )
            }
            if (!operation.stdout.isNullOrBlank() || !operation.stderr.isNullOrBlank()) {
                SelectionContainer {
                    Text(listOfNotNull(operation.stdout, operation.stderr).joinToString("\n").takeLast(24_000),
                        color = TextPrimary, fontFamily = FontFamily.Monospace)
                }
            }
        }
    }
}
