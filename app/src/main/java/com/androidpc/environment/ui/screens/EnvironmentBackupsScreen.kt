package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.BackupData
import com.androidpc.environment.bridge.EnvironmentConfigData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.launch
import java.text.DateFormat
import java.util.Date
import java.util.UUID

@Composable
fun EnvironmentBackupsScreen(
    source: EnvironmentConfigData?,
    onBack: () -> Unit,
    onRestored: (String) -> Unit,
) {
    val scope = rememberCoroutineScope()
    var backups by remember(source?.id) { mutableStateOf<List<BackupData>>(emptyList()) }
    var busy by remember { mutableStateOf<String?>(null) }
    var cancelId by remember { mutableStateOf<String?>(null) }
    var restore by remember { mutableStateOf<BackupData?>(null) }
    var clonePrompt by remember { mutableStateOf(false) }
    var name by remember { mutableStateOf("") }
    var status by remember { mutableStateOf<String?>(null) }
    suspend fun refresh() {
        NativeBridge.backupOperation("list", source = source?.id.orEmpty())
            .onSuccess { backups = it.backups.orEmpty() }
            .onFailure { status = it.message }
    }
    LaunchedEffect(source?.id) { refresh() }

    fun start(operation: String, label: String, backup: String = UUID.randomUUID().toString()) {
        val sourceId = source?.id.orEmpty()
        val targetId = if (operation == "create") "" else UUID.randomUUID().toString()
        val submittedName = name.trim()
        busy = label
        status = null
        cancelId = if (targetId.isEmpty()) sourceId else targetId
        scope.launch {
            try {
                NativeBridge.backupOperation(operation, sourceId, backup, targetId, submittedName)
                    .onSuccess { result ->
                        result.environment_id?.let(onRestored) ?: run {
                            status = "Backup saved"
                            refresh()
                        }
                    }.onFailure { status = it.message ?: "Operation failed" }
            } finally {
                busy = null
                cancelId = null
            }
        }
    }

    if (restore != null || clonePrompt) {
        val backup = restore
        AlertDialog(
            onDismissRequest = { restore = null; clonePrompt = false },
            title = { Text(if (backup == null) "Clone environment" else "Restore as a new environment") },
            text = {
                OutlinedTextField(
                    value = name, onValueChange = { if (it.length <= 80) name = it },
                    label = { Text("New environment name") }, singleLine = true,
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    restore = null
                    clonePrompt = false
                    if (backup == null) start("clone", "Cloning environment")
                    else start("restore", "Restoring backup", backup.id)
                }, enabled = name.isNotBlank()) { Text(if (backup == null) "Clone" else "Restore") }
            },
            dismissButton = { TextButton(onClick = { restore = null; clonePrompt = false }) { Text("Cancel") } }
        )
    }
    LazyColumn(
        Modifier.fillMaxSize().background(BgDark).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        item {
            TextButton(onClick = onBack, enabled = busy == null) { Text("← Back", color = AccentCyan) }
            Text("Backups", style = MaterialTheme.typography.headlineSmall, color = TextPrimary)
        }
        if (source != null) {
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Button(onClick = { start("create", "Backing up environment") }, enabled = busy == null) {
                        Text("Create backup")
                    }
                    OutlinedButton(onClick = {
                        name = ((source.display_name ?: source.distribution) + " copy").take(80)
                        clonePrompt = true
                    }, enabled = busy == null) { Text("Clone environment") }
                }
                Text("Back up and clone stopped environments.", color = TextSecondary)
            }
        }
        busy?.let { label ->
            item {
                Text(label, color = TextPrimary)
                LinearProgressIndicator(Modifier.fillMaxWidth())
                TextButton(onClick = {
                    scope.launch {
                        val target = cancelId
                        val cancelled = target != null && NativeBridge.cancelPackageOperation(target)
                        if (!cancelled && source != null && target != source.id) {
                            NativeBridge.cancelPackageOperation(source.id)
                        }
                    }
                }) { Text("Cancel operation") }
            }
        }
        status?.let { message -> item { Text(message, color = TextSecondary) } }
        if (backups.isEmpty() && busy == null) item { Text("No backups yet.", color = TextSecondary) }
        items(backups, key = { it.id }) { backup ->
            Card(colors = CardDefaults.cardColors(containerColor = SurfaceDark)) {
                Row(Modifier.fillMaxWidth().padding(12.dp), horizontalArrangement = Arrangement.SpaceBetween) {
                    Column(Modifier.weight(1f)) {
                        Text(backup.source_name, color = TextPrimary)
                        val date = DateFormat.getDateTimeInstance().format(Date(backup.created_at_epoch_ms))
                        Text("$date · ${backup.size_bytes / (1024 * 1024)} MiB", color = TextSecondary)
                    }
                    TextButton(onClick = {
                        name = (backup.source_name + " restored").take(80)
                        restore = backup
                    }, enabled = busy == null) { Text("Restore") }
                }
            }
        }
    }
}
