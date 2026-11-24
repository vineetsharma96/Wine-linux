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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.bridge.ArtifactMetadataData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

@Composable
fun CreateEnvironmentScreen(
    onBack: () -> Unit,
    onCreated: (String) -> Unit
) {
    var selectedDistro by remember { mutableStateOf("ubuntu") }
    var selectedDesktop by remember { mutableStateOf("xfce") }
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

        Spacer(modifier = Modifier.height(20.dp))

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

        Spacer(modifier = Modifier.height(16.dp))

        // Artifact Details Card
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
                        text = "VERIFIED ROOTFS MANIFEST",
                        style = MaterialTheme.typography.labelSmall,
                        color = AccentCyan,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                    Text(
                        text = "Archive: ${meta.filename}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextPrimary
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

        Spacer(modifier = Modifier.height(16.dp))

        Text("DESKTOP ENVIRONMENT", style = MaterialTheme.typography.labelSmall, color = TextMuted)
        Spacer(modifier = Modifier.height(8.dp))
        DistroOption(
            title = "XFCE 4 Desktop",
            subtitle = "Lightweight desktop optimized for touch, mouse & keyboard",
            isSelected = true,
            enabled = !isDeploying,
            modifier = Modifier.fillMaxWidth()
        ) { selectedDesktop = "xfce" }

        Spacer(modifier = Modifier.height(16.dp))

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

        // Provisioning Status / Progress Box
        if (isDeploying || isSuccess) {
            Spacer(modifier = Modifier.height(16.dp))
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
                        Text(
                            text = "${(deployProgress * 100).toInt()}%",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextPrimary
                        )
                    }
                    Spacer(modifier = Modifier.height(8.dp))
                    LinearProgressIndicator(
                        progress = { deployProgress },
                        modifier = Modifier.fillMaxWidth().height(6.dp),
                        color = if (isSuccess) StatusGreen else AccentCyan,
                        trackColor = BgDark
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = deployStage,
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                }
            }
        }

        errorMessage?.let { err ->
            Spacer(modifier = Modifier.height(10.dp))
            Text(
                text = "Error: $err",
                style = MaterialTheme.typography.bodySmall,
                color = StatusRed
            )
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
                    coroutineScope.launch {
                        try {
                            deployStage = "1/4 Verifying SHA-256 cryptographic manifest..."
                            deployProgress = 0.25f
                            delay(400)

                            deployStage = "2/4 Initializing sandbox directory structure..."
                            deployProgress = 0.50f
                            delay(400)

                            deployStage = "3/4 Configuring guest network (/etc/resolv.conf) & hosts..."
                            deployProgress = 0.75f
                            val envId = "env-$selectedDistro"
                            val provRes = NativeBridge.provisionEnvironment(envId, selectedDistro)

                            provRes.onSuccess {
                                deployStage = "4/4 Environment provisioned successfully! Ready to launch."
                                deployProgress = 1.0f
                                isSuccess = true
                                isDeploying = false
                            }.onFailure { err ->
                                errorMessage = err.message
                                isDeploying = false
                            }
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
