package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.ArtifactMetadataData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.bridge.downloadRootfs
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import java.io.File
import java.util.UUID

@Composable
fun CreateEnvironmentScreen(
    onBack: () -> Unit,
    onCreated: (String) -> Unit
) {
    val context = LocalContext.current
    var selectedDistro by rememberSaveable { mutableStateOf("ubuntu") }
    var environmentName by rememberSaveable { mutableStateOf("") }
    val envId = rememberSaveable { UUID.randomUUID().toString() }
    var artifactInfo by remember { mutableStateOf<ArtifactMetadataData?>(null) }

    var isDeploying by remember { mutableStateOf(false) }
    var deployStage by remember { mutableStateOf("") }
    var deployProgress by remember { mutableFloatStateOf(0f) }
    var isSuccess by remember { mutableStateOf(false) }
    var errorMessage by remember { mutableStateOf<String?>(null) }
    var downloadJob by remember { mutableStateOf<Job?>(null) }
    var nativeInstalling by remember { mutableStateOf(false) }
    var cancelling by remember { mutableStateOf(false) }

    val coroutineScope = rememberCoroutineScope()

    LaunchedEffect(selectedDistro) {
        artifactInfo = null
        errorMessage = null
        NativeBridge.getArtifactInfo(selectedDistro)
            .onSuccess { artifactInfo = it }
            .onFailure { errorMessage = it.message }
    }

    LaunchedEffect(nativeInstalling, envId) {
        while (nativeInstalling && isActive) {
            when (NativeBridge.getEnvironmentState(envId)) {
                "Verifying" -> {
                    deployStage = "Verifying archive"
                    deployProgress = 0.78f
                }
                "Extracting" -> {
                    deployStage = "Extracting Linux filesystem"
                    deployProgress = 0.85f
                }
                "Initializing" -> {
                    deployStage = "Checking the Linux guest"
                    deployProgress = 0.95f
                }
            }
            delay(100)
        }
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

        OutlinedTextField(
            value = environmentName,
            onValueChange = { if (it.length <= 80) environmentName = it },
            label = { Text("Environment name") },
            placeholder = { Text(if (selectedDistro == "ubuntu") "Ubuntu" else "Debian") },
            enabled = !isDeploying && !isSuccess,
            singleLine = true,
            modifier = Modifier.fillMaxWidth()
        )
        Spacer(modifier = Modifier.height(12.dp))

        Text("SELECT DISTRIBUTION", style = MaterialTheme.typography.labelSmall, color = TextMuted)
        Spacer(modifier = Modifier.height(8.dp))

        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            DistroOption(
                title = "Ubuntu",
                subtitle = "26.04.1 LTS • Single catalog release",
                isSelected = selectedDistro == "ubuntu",
                enabled = !isDeploying && !isSuccess,
                modifier = Modifier.weight(1f)
            ) { selectedDistro = "ubuntu" }

            DistroOption(
                title = "Debian",
                subtitle = "13.8 • Single catalog release",
                isSelected = selectedDistro == "debian",
                enabled = !isDeploying && !isSuccess,
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
                        text = "Release: ${meta.distribution.replaceFirstChar(Char::uppercase)} (single signed release in catalog)",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextSecondary
                    )
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
                    Text(if (cancelling) "Cancelling installation…" else deployStage, style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                    downloadJob?.let { job ->
                        TextButton(onClick = { job.cancel() }) { Text("Cancel download") }
                    }
                    if (nativeInstalling && !cancelling) {
                        TextButton(onClick = {
                            coroutineScope.launch { cancelling = NativeBridge.cancelInstallation(envId) }
                        }) { Text("Cancel installation") }
                    }
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
                onClick = { onCreated(envId) },
                modifier = Modifier
                    .fillMaxWidth()
                    .height(50.dp),
                colors = ButtonDefaults.buttonColors(containerColor = StatusGreen),
                shape = RoundedCornerShape(12.dp)
            ) {
                Text("Open Linux Terminal", color = BgDark, fontWeight = FontWeight.Bold)
            }
        } else {
            Button(
                onClick = {
                    val meta = artifactInfo ?: return@Button
                    isDeploying = true
                    errorMessage = null
                    deployProgress = 0f
                    downloadJob = coroutineScope.launch {
                        val partFile = File(context.cacheDir, meta.filename + ".part")
                        try {
                            check(android.os.Build.SUPPORTED_ABIS.contains("arm64-v8a")) {
                                "This Linux runtime requires an ARM64 device"
                            }
                            deployStage = "Downloading " + meta.filename
                            downloadRootfs(meta, partFile, context.filesDir) { fraction ->
                                deployProgress = fraction * 0.75f
                            }
                            downloadJob = null
                            nativeInstalling = true
                            deployStage = "Checking the archive"
                            val name = environmentName.trim().ifEmpty { if (selectedDistro == "ubuntu") "Ubuntu" else "Debian" }
                            val result = NativeBridge.installEnvironment(envId, selectedDistro, 2048, partFile, name)
                            if (!cancelling) partFile.delete()
                            result.getOrThrow()
                            deployStage = "Linux shell verified and ready"
                            deployProgress = 1f
                            isSuccess = true
                        } catch (e: CancellationException) {
                            throw e
                        } catch (e: Exception) {
                            errorMessage = e.message ?: "Environment installation failed"
                        } finally {
                            isDeploying = false
                            downloadJob = null
                            nativeInstalling = false
                            cancelling = false
                        }
                    }
                },
                enabled = !isDeploying && artifactInfo != null,
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
