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
import com.androidpc.environment.BuildConfig
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

    val pinnedStack = remember(artifactsDir) {
        listOf(
            Triple("Box64 (x86-64 Dynarec)", "v${BuildConfig.BOX64_VERSION} • MIT", File(artifactsDir, "box64/box64").isFile),
            Triple("Wine (Pure-64 WoW64)", "v${BuildConfig.WINE_VERSION} • LGPL-2.1+", File(artifactsDir, "wine/bin/wine").isFile),
            Triple("Mesa Turnip (Adreno Vulkan)", "v${BuildConfig.TURNIP_VERSION} • MIT", File(artifactsDir, "graphics/turnip/libvulkan_freedreno.so").isFile),
            Triple("VirGL (virglrenderer OpenGL)", "v${BuildConfig.VIRGL_VERSION} • MIT", File(artifactsDir, "graphics/virgl/virgl_test_server_android").isFile),
            Triple("DXVK (D3D8–11 → Vulkan)", "v${BuildConfig.DXVK_VERSION} • zlib", File(artifactsDir, "dxvk/x64/d3d11.dll").isFile),
            Triple("VKD3D-Proton (D3D12 → Vulkan)", "v${BuildConfig.VKD3D_VERSION} • LGPL-2.1", File(artifactsDir, "vkd3d-proton/x64/d3d12.dll").isFile),
            Triple("QEMU VM Engine (ARM64 + x86-64)", "v${BuildConfig.QEMU_VERSION} • GPL-2.0 • libslirp ${BuildConfig.LIBSLIRP_VERSION}", File(artifactsDir, "qemu/qemu-system-aarch64").isFile),
            Triple("EDK2 UEFI Firmware (AAVMF + OVMF)", "${BuildConfig.EDK2_VERSION} • BSD-2-Clause-Patent", File(artifactsDir, "qemu-firmware/AAVMF_CODE.fd").isFile)
        )
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

        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, AccentPurple.copy(alpha = 0.55f), RoundedCornerShape(12.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceElevated),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        text = "PINNED RUNTIME & VIRTUALIZATION STACK",
                        style = MaterialTheme.typography.labelSmall,
                        color = AccentCyan,
                        fontWeight = FontWeight.Bold
                    )
                    pinnedStack.forEach { (label, versionInfo, binaryPresent) ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(label, style = MaterialTheme.typography.bodyMedium, color = TextPrimary, fontWeight = FontWeight.SemiBold)
                                Text(versionInfo, style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                            }
                            Text(
                                text = if (binaryPresent) "STAGED" else "MANIFEST PACKAGED",
                                style = MaterialTheme.typography.labelSmall,
                                color = if (binaryPresent) StatusGreen else StatusAmber,
                                fontWeight = FontWeight.Bold
                            )
                        }
                    }
                }
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
