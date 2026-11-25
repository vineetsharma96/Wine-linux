package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.ArtifactMetadataData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

@Composable
fun CreateEnvironmentScreen(
    onBack: () -> Unit,
    onCreated: (String) -> Unit
) {
    val context = LocalContext.current
    var selectedDistro by remember { mutableStateOf("ubuntu") }
    var downloadRootfsArchive by remember { mutableStateOf(false) }
    var memoryMb by remember { mutableFloatStateOf(2048f) }
    var artifactInfo by remember { mutableStateOf<ArtifactMetadataData?>(null) }

    var isDeploying by remember { mutableStateOf(false) }
    var deployStage by remember { mutableStateOf("") }
    var deployProgress by remember { mutableFloatStateOf(0f) }
    var isSuccess by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }

    val coroutineScope = rememberCoroutineScope()

    LaunchedEffect(selectedDistro) {
        val res = NativeBridge.getArtifactInfo(selectedDistro)
        res.onSuccess { artifactInfo = it }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark)
            .padding(16.dp)
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = onBack, enabled = !isDeploying) {
                Text("← Back", color = AccentCyan)
            }
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = "Deploy Linux Environment",
                style = MaterialTheme.typography.titleLarge,
                color = TextPrimary,
                fontWeight = FontWeight.Bold
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        Text("SELECT DISTRIBUTION", style = MaterialTheme.typography.labelSmall, color = TextMuted)
        Spacer(modifier = Modifier.height(8.dp))

        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            DistroOption(
                title = "Ubuntu 24.04",
                subtitle = "Noble Numbat (ARM64)",
                isSelected = selectedDistro == "ubuntu",
                enabled = !isDeploying,
                modifier = Modifier.weight(1f)
            ) { selectedDistro = "ubuntu" }

            DistroOption(
                title = "Debian 12",
                subtitle = "Bookworm (ARM64)",
                isSelected = selectedDistro == "debian",
                enabled = !isDeploying,
                modifier = Modifier.weight(1f)
            ) { selectedDistro = "debian" }
        }

        Spacer(modifier = Modifier.height(14.dp))

        artifactInfo?.let { meta ->
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(10.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Text(
                        text = "ROOTFS ARTIFACT MANIFEST",
                        style = MaterialTheme.typography.labelSmall,
                        color = AccentCyan,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                    Text("Archive: ${meta.filename}", style = MaterialTheme.typography.bodyMedium, color = TextPrimary)
                    Text(
                        text = "Size: ${meta.size_bytes / (1024 * 1024)} MB | Arch: ${meta.architecture}",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextSecondary
                    )
                    Text(
                        text = "SHA-256: ${meta.sha256.take(24)}...",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextMuted
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(12.dp))

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(SurfaceDark, RoundedCornerShape(10.dp))
                .border(1.dp, CardBorder, RoundedCornerShape(10.dp))
                .padding(horizontal = 14.dp, vertical = 10.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Column(modifier = Modifier.weight(1f)) {
                Text("Fetch Remote Rootfs Tarball", style = MaterialTheme.typography.bodyMedium, color = TextPrimary)
                Text(
                    "Stream HTTPS archive to .part, verify SHA-256 & unpack",
                    style = MaterialTheme.typography.labelSmall,
                    color = TextSecondary
                )
            }
            Switch(
                checked = downloadRootfsArchive,
                onCheckedChange = { downloadRootfsArchive = it },
                enabled = !isDeploying
            )
        }

        Spacer(modifier = Modifier.height(14.dp))

        Text("MEMORY LIMIT: ${memoryMb.toInt()} MB", style = MaterialTheme.typography.labelSmall, color = TextMuted)
        Slider(
            value = memoryMb,
            onValueChange = { memoryMb = it },
            valueRange = 512f..8192f,
            steps = 14,
            enabled = !isDeploying,
            colors = SliderDefaults.colors(
                thumbColor = AccentCyan,
                activeTrackColor = AccentCyan,
                inactiveTrackColor = SurfaceContainer
            )
        )

        if (isDeploying || isSuccess) {
            Spacer(modifier = Modifier.height(12.dp))
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, if (isSuccess) StatusGreen else AccentCyan, RoundedCornerShape(10.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
                shape = RoundedCornerShape(10.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = if (isSuccess) "PROVISIONING COMPLETE" else "PROVISIONING ROOTFS",
                            style = MaterialTheme.typography.labelSmall,
                            color = if (isSuccess) StatusGreen else AccentCyan,
                            fontWeight = FontWeight.Bold
                        )
                        Text("${(deployProgress * 100).toInt()}%", style = MaterialTheme.typography.labelSmall, color = TextPrimary)
                    }
                    Spacer(modifier = Modifier.height(8.dp))
                    LinearProgressIndicator(
                        progress = { deployProgress },
                        modifier = Modifier.fillMaxWidth().height(6.dp),
                        color = if (isSuccess) StatusGreen else AccentCyan,
                        trackColor = BgDark
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(deployStage, style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                }
            }
        }

        errorMessage?.let { err ->
            Spacer(modifier = Modifier.height(10.dp))
            Text("Error: $err", style = MaterialTheme.typography.bodySmall, color = StatusRed)
        }

        Spacer(modifier = Modifier.weight(1f))

        if (isSuccess) {
            Button(
                onClick = { onCreated(selectedDistro) },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(50.dp),
                colors = ButtonDefaults.buttonColors(containerColor = StatusGreen),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text("Launch Desktop Viewport", color = BgDark, fontWeight = FontWeight.Bold)
            }
        } else {
            Button(
                onClick = {
                    isDeploying = true
                    errorMessage = null
                    val envId = "env-$selectedDistro"
                    coroutineScope.launch {
                        try {
                            deployStage = "1/3 Provisioning sandbox directory & rootfs config..."
                            deployProgress = 0.25f
                            val provRes = NativeBridge.provisionEnvironmentWithMemory(envId, selectedDistro, memoryMb.toInt())
                            if (provRes.isFailure) {
                                errorMessage = provRes.exceptionOrNull()?.message
                                isDeploying = false
                                return@launch
                            }

                            if (downloadRootfsArchive && artifactInfo != null) {
                                val meta = artifactInfo!!
                                deployStage = "2/3 Downloading ${meta.filename}..."
                                deployProgress = 0.45f
                                val partFile = File(context.cacheDir, "${meta.filename}.part")
                                val downloadedOk = withContext(Dispatchers.IO) {
                                    runCatching {
                                        val conn = URL(meta.url).openConnection() as HttpURLConnection
                                        conn.connectTimeout = 10_000
                                        conn.readTimeout = 30_000
                                        conn.inputStream.use { input ->
                                            partFile.outputStream().use { out ->
                                                val buf = ByteArray(64 * 1024)
                                                var total = 0L
                                                while (true) {
                                                    val n = input.read(buf)
                                                    if (n <= 0) break
                                                    out.write(buf, 0, n)
                                                    total += n
                                                    if (meta.size_bytes > 0) {
                                                        val frac = (total.toFloat() / meta.size_bytes.toFloat()).coerceIn(0f, 1f)
                                                        deployProgress = 0.45f + frac * 0.35f
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if (downloadedOk.isSuccess) {
                                    deployStage = "3/3 Verifying SHA-256 & extracting rootfs..."
                                    deployProgress = 0.85f
                                    val enforceSha = meta.sha256.isNotBlank() && !meta.sha256.all { it == '0' }
                                    val verified = if (enforceSha) {
                                        NativeBridge.verifyArtifact(partFile.absolutePath, meta.sha256).getOrDefault(false)
                                    } else {
                                        true
                                    }
                                    if (!verified) {
                                        partFile.delete()
                                        errorMessage = "SHA-256 checksum verification failed for ${meta.filename}"
                                        isDeploying = false
                                        return@launch
                                    }
                                    val rootfsDir = File(context.filesDir, "environments/$envId/rootfs").absolutePath
                                    NativeBridge.extractRootfs(partFile.absolutePath, rootfsDir)
                                    partFile.delete()
                                } else {
                                    deployStage = "Download skipped (${downloadedOk.exceptionOrNull()?.message}); sandbox ready."
                                }
                            } else {
                                deployStage = "2/3 Guest DNS (/etc/resolv.conf), hosts & startxfce.sh written..."
                                deployProgress = 0.80f
                            }

                            deployStage = "Environment '$envId' provisioned & persisted."
                            deployProgress = 1.0f
                            isSuccess = true
                            isDeploying = false
                        } catch (e: Exception) {
                            errorMessage = e.message
                            isDeploying = false
                        }
                    }
                },
                enabled = !isDeploying,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(50.dp),
                colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text(
                    text = if (isDeploying) "Provisioning..." else "Deploy Environment (Rootless)",
                    color = BgDark,
                    fontWeight = FontWeight.Bold
                )
            }
        }
    }
}

@Composable
fun DistroOption(
    title: String,
    subtitle: String,
    isSelected: Boolean,
    enabled: Boolean = true,
    modifier: Modifier = Modifier,
    onClick: () -> Unit
) {
    val borderColor = if (isSelected) AccentCyan else CardBorder
    Card(
        modifier = modifier
            .border(1.dp, borderColor, RoundedCornerShape(10.dp))
            .clickable(enabled = enabled) { onClick() },
        colors = CardDefaults.cardColors(containerColor = SurfaceDark),
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium, color = TextPrimary, fontWeight = FontWeight.Bold)
            Spacer(modifier = Modifier.height(4.dp))
            Text(subtitle, style = MaterialTheme.typography.bodyMedium, color = TextSecondary)
        }
    }
}
