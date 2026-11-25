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
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.AudioStatusData
import com.androidpc.environment.bridge.LogEntryData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.launch

@Composable
fun SettingsScreen(
    displayResolution: Pair<Int, Int>,
    audioEnabled: Boolean,
    wineArch: String,
    onResolutionChange: (Pair<Int, Int>) -> Unit,
    onAudioEnabledChange: (Boolean) -> Unit,
    onWineArchChange: (String) -> Unit,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var logs by remember { mutableStateOf<List<LogEntryData>>(emptyList()) }
    var audioStatus by remember { mutableStateOf<AudioStatusData?>(null) }

    val refreshSettingsTelemetry: () -> Unit = {
        scope.launch {
            NativeBridge.getRecentLogs(30).onSuccess { logs = it.reversed() }
            NativeBridge.getAudioStatus().onSuccess { audioStatus = it }
        }
    }

    LaunchedEffect(Unit) {
        refreshSettingsTelemetry()
    }

    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    TextButton(onClick = onBack) {
                        Text("← Back", color = AccentCyan)
                    }
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(
                        text = "Settings & Logs",
                        style = MaterialTheme.typography.titleLarge,
                        color = TextPrimary,
                        fontWeight = FontWeight.Bold
                    )
                }
                TextButton(onClick = refreshSettingsTelemetry) {
                    Text("Refresh", color = AccentCyan)
                }
            }
        }

        // Display Resolution Selector
        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(10.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text("Display Resolution (Xvnc)", style = MaterialTheme.typography.titleMedium, color = TextPrimary)
                    Spacer(modifier = Modifier.height(8.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        listOf(1280 to 720, 1600 to 900, 1920 to 1080).forEach { res ->
                            val selected = displayResolution == res
                            OutlinedButton(
                                onClick = { onResolutionChange(res) },
                                colors = ButtonDefaults.outlinedButtonColors(
                                    containerColor = if (selected) AccentCyan.copy(alpha = 0.2f) else SurfaceContainer
                                ),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text(
                                    "${res.first}x${res.second}",
                                    color = if (selected) AccentCyan else TextSecondary,
                                    style = MaterialTheme.typography.labelSmall
                                )
                            }
                        }
                    }
                }
            }
        }

        // Audio TCP Sink Toggle
        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(10.dp)
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(14.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text("PulseAudio TCP Sink (127.0.0.1:4713)", style = MaterialTheme.typography.titleMedium, color = TextPrimary)
                        Text(
                            text = if (audioStatus?.running == true) {
                                "Listening • ${audioStatus?.total_bytes_received ?: 0} bytes received"
                            } else {
                                "Streams Linux/Wine audio to Android AudioTrack"
                            },
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                    }
                    Switch(
                        checked = audioEnabled,
                        onCheckedChange = onAudioEnabledChange
                    )
                }
            }
        }

        // Wine Architecture Selector
        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(10.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text("Wine Default Architecture", style = MaterialTheme.typography.titleMedium, color = TextPrimary)
                    Spacer(modifier = Modifier.height(8.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        listOf("win64", "wow64", "win32").forEach { arch ->
                            val selected = wineArch == arch
                            OutlinedButton(
                                onClick = { onWineArchChange(arch) },
                                colors = ButtonDefaults.outlinedButtonColors(
                                    containerColor = if (selected) AccentCyan.copy(alpha = 0.2f) else SurfaceContainer
                                ),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text(
                                    arch.uppercase(),
                                    color = if (selected) AccentCyan else TextSecondary,
                                    style = MaterialTheme.typography.labelSmall
                                )
                            }
                        }
                    }
                }
            }
        }

        // Live Rust Engine Logs
        item {
            Text(
                text = "RUST ENGINE LOG RING BUFFER (${logs.size})",
                style = MaterialTheme.typography.labelSmall,
                color = TextMuted
            )
        }

        items(logs) { entry ->
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(8.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(8.dp)
            ) {
                Column(modifier = Modifier.padding(10.dp)) {
                    Text(
                        text = "[${entry.level}] ${entry.target}",
                        style = MaterialTheme.typography.labelSmall,
                        color = if (entry.level == "ERROR") StatusRed else AccentCyan
                    )
                    Text(
                        text = entry.message,
                        color = TextPrimary,
                        fontFamily = FontFamily.Monospace,
                        fontSize = 11.sp
                    )
                }
            }
        }
    }
}
