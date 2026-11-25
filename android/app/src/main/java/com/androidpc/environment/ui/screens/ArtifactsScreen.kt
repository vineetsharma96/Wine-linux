package com.androidpc.environment.ui.screens

import androidx.compose.foundation.BorderStroke
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
fun ArtifactsScreen(
    onBack: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var artifacts by remember { mutableStateOf<List<ArtifactMetadataData>>(emptyList()) }
    var activeDownload by remember { mutableStateOf<String?>(null) }
    var downloadProgress by remember { mutableFloatStateOf(0f) }
    var statusMessage by remember { mutableStateOf<String?>(null) }

    val artifactsDir = remember { File(context.filesDir, "artifacts") }

    LaunchedEffect(Unit) {
        NativeBridge.listArtifacts().onSuccess { artifacts = it }
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
                verticalAlignment = Alignment.CenterVertically
            ) {
                TextButton(onClick = onBack, enabled = activeDownload == null) {
                    Text("← Back", color = AccentCyan)
                }
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "Runtime Artifacts",
                    style = MaterialTheme.typography.titleLarge,
                    color = TextPrimary,
                    fontWeight = FontWeight.Bold
                )
            }
        }

        statusMessage?.let { msg ->
            item {
                Text(msg, style = MaterialTheme.typography.bodySmall, color = AccentCyan)
            }
        }

        items(artifacts, key = { it.name }) { meta ->
            val installedFile = File(artifactsDir, meta.filename)
            val isInstalled = installedFile.exists() && installedFile.length() > 0L
            val isDownloadingThis = activeDownload == meta.name

            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, if (isInstalled) StatusGreen else CardBorder, RoundedCornerShape(10.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
                shape = RoundedCornerShape(10.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(meta.name, style = MaterialTheme.typography.titleMedium, color = TextPrimary, fontWeight = FontWeight.Bold)
                        Text(
                            text = if (isInstalled) "VERIFIED" else meta.architecture.uppercase(),
                            style = MaterialTheme.typography.labelSmall,
                            color = if (isInstalled) StatusGreen else TextSecondary,
                            fontWeight = FontWeight.Bold
                        )
                    }
                    Spacer(modifier = Modifier.height(4.dp))
                    Text("File: ${meta.filename} (${meta.size_bytes / 1024} KB)", style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                    Text("SHA-256: ${meta.sha256.take(28)}...", style = MaterialTheme.typography.labelSmall, color = TextMuted)

                    if (isDownloadingThis) {
                        Spacer(modifier = Modifier.height(8.dp))
                        LinearProgressIndicator(
                            progress = { downloadProgress },
                            modifier = Modifier.fillMaxWidth().height(6.dp),
                            color = AccentCyan,
                            trackColor = BgDark
                        )
                    }

                    Spacer(modifier = Modifier.height(10.dp))

                    Button(
                        onClick = {
                            activeDownload = meta.name
                            downloadProgress = 0.05f
                            statusMessage = "Downloading ${meta.filename}..."
                            scope.launch {
                                val partFile = File(context.cacheDir, "${meta.filename}.part")
                                val dlResult = withContext(Dispatchers.IO) {
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
                                                        downloadProgress = (total.toFloat() / meta.size_bytes.toFloat()).coerceIn(0.05f, 0.95f)
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if (dlResult.isSuccess) {
                                    val installRes = NativeBridge.verifyAndInstallArtifact(
                                        partFile.absolutePath,
                                        meta.filename,
                                        meta.sha256
                                    )
                                    statusMessage = if (installRes.isSuccess) {
                                        "Verified & installed ${meta.filename}"
                                    } else {
                                        partFile.delete()
                                        "SHA-256 verification failed: ${installRes.exceptionOrNull()?.message}"
                                    }
                                } else {
                                    partFile.delete()
                                    statusMessage = "Download failed: ${dlResult.exceptionOrNull()?.message}"
                                }
                                activeDownload = null
                            }
                        },
                        enabled = activeDownload == null,
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                        border = BorderStroke(1.dp, CardBorder),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text(
                            text = if (isDownloadingThis) "Downloading..." else if (isInstalled) "Re-verify & Update" else "Download & Verify SHA-256",
                            color = AccentCyan
                        )
                    }
                }
            }
        }
    }
}
