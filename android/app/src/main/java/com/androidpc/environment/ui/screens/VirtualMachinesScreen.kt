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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.KvmCapabilitiesData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.bridge.VmStatusData
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.launch

@Composable
fun VirtualMachinesScreen(
    onOpenVmDisplay: (Int) -> Unit,
    onBack: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var kvmCaps by remember { mutableStateOf<KvmCapabilitiesData?>(null) }
    var vms by remember { mutableStateOf<List<VmStatusData>>(emptyList()) }
    var statusMessage by remember { mutableStateOf<String?>(null) }
    var showCreateDialog by remember { mutableStateOf(false) }

    var newVmId by remember { mutableStateOf("vm-linux-arm64") }
    var newVmArch by remember { mutableStateOf("aarch64") }
    var newVmMemoryMb by remember { mutableFloatStateOf(1024f) }
    var newVmCpus by remember { mutableFloatStateOf(2f) }
    var newVmDiskMb by remember { mutableFloatStateOf(2048f) }

    val refreshVms: () -> Unit = {
        scope.launch {
            NativeBridge.probeKvmSupport().onSuccess { kvmCaps = it }
            NativeBridge.listVms().onSuccess { vms = it }
        }
    }

    LaunchedEffect(Unit) {
        refreshVms()
    }

    if (showCreateDialog) {
        AlertDialog(
            onDismissRequest = { showCreateDialog = false },
            containerColor = SurfaceDark,
            title = { Text("Create QEMU Virtual Machine", color = TextPrimary) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    OutlinedTextField(
                        value = newVmId,
                        onValueChange = { newVmId = it },
                        label = { Text("VM Identifier", color = TextSecondary) },
                        singleLine = true,
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = AccentCyan,
                            unfocusedBorderColor = CardBorder
                        )
                    )
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        listOf("aarch64", "x86_64").forEach { arch ->
                            FilterChip(
                                selected = newVmArch == arch,
                                onClick = { newVmArch = arch },
                                label = { Text(arch) }
                            )
                        }
                    }
                    Text("RAM: ${newVmMemoryMb.toInt()} MB", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                    Slider(
                        value = newVmMemoryMb,
                        onValueChange = { newVmMemoryMb = it },
                        valueRange = 512f..4096f,
                        steps = 6
                    )
                    Text("vCPUs: ${newVmCpus.toInt()} | Disk: ${newVmDiskMb.toInt()} MB", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                    Slider(
                        value = newVmCpus,
                        onValueChange = { newVmCpus = it },
                        valueRange = 1f..8f,
                        steps = 6
                    )
                }
            },
            confirmButton = {
                Button(
                    onClick = {
                        showCreateDialog = false
                        scope.launch {
                            NativeBridge.createVm(
                                vmId = newVmId.trim(),
                                arch = newVmArch,
                                memoryMb = newVmMemoryMb.toInt(),
                                cpus = newVmCpus.toInt(),
                                diskSizeMb = newVmDiskMb.toInt()
                            ).onSuccess {
                                statusMessage = "Created VM '${newVmId.trim()}'"
                                refreshVms()
                            }.onFailure {
                                statusMessage = "Create VM error: ${it.message}"
                            }
                        }
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = AccentCyan)
                ) {
                    Text("Create", color = BgDark, fontWeight = FontWeight.Bold)
                }
            },
            dismissButton = {
                TextButton(onClick = { showCreateDialog = false }) {
                    Text("Cancel", color = TextSecondary)
                }
            }
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
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    TextButton(onClick = onBack) {
                        Text("← Back", color = AccentCyan)
                    }
                    Text(
                        text = "QEMU Virtual Machines",
                        style = MaterialTheme.typography.titleLarge,
                        color = TextPrimary,
                        fontWeight = FontWeight.Bold
                    )
                }
                Button(
                    onClick = { showCreateDialog = true },
                    colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                    shape = RoundedCornerShape(8.dp)
                ) {
                    Text("+ New VM", color = BgDark, fontWeight = FontWeight.Bold)
                }
            }
        }

        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(10.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp)) {
                    val accelLabel = if (kvmCaps?.accessible == true) "KVM HARDWARE ACCEL" else "TCG SOFTWARE EMULATION"
                    val accelColor = if (kvmCaps?.accessible == true) StatusGreen else StatusAmber
                    Text(
                        text = accelLabel,
                        style = MaterialTheme.typography.labelSmall,
                        color = accelColor,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = kvmCaps?.notes ?: "Probing /dev/kvm...",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                }
            }
        }

        statusMessage?.let { msg ->
            item {
                Text(msg, style = MaterialTheme.typography.bodySmall, color = AccentCyan)
            }
        }

        if (vms.isEmpty()) {
            item {
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("No Virtual Machines Configured", color = TextPrimary, fontWeight = FontWeight.SemiBold)
                        Spacer(modifier = Modifier.height(4.dp))
                        Text(
                            "Tap '+ New VM' to allocate a sparse VirtIO disk image and stage QEMU (aarch64 or x86_64).",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                    }
                }
            }
        } else {
            items(vms, key = { it.id }) { vm ->
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Column(modifier = Modifier.padding(14.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text(vm.id, style = MaterialTheme.typography.titleMedium, color = TextPrimary, fontWeight = FontWeight.Bold)
                            Text(
                                text = "${vm.state} (${vm.accelerator.uppercase()})",
                                style = MaterialTheme.typography.labelSmall,
                                color = if (vm.state == "Running") StatusGreen else TextSecondary
                            )
                        }
                        Spacer(modifier = Modifier.height(6.dp))
                        Text(
                            text = "Arch: ${vm.arch} | RAM: ${vm.memory_mb} MB | vCPU: ${vm.cpus} | VNC: :${vm.vnc_port}",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                        Text(
                            text = "QMP: 127.0.0.1:${vm.qmp_port} | SSH Forward: 127.0.0.1:2222 -> :22",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted
                        )
                        Spacer(modifier = Modifier.height(10.dp))
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            Button(
                                onClick = {
                                    scope.launch {
                                        if (vm.state == "Running") {
                                            NativeBridge.stopVm(vm.id)
                                                .onSuccess { statusMessage = "Stopped '${vm.id}'" }
                                                .onFailure { statusMessage = it.message }
                                        } else {
                                            NativeBridge.startVm(vm.id)
                                                .onSuccess { statusMessage = "Started '${vm.id}'" }
                                                .onFailure { statusMessage = it.message }
                                        }
                                        refreshVms()
                                    }
                                },
                                colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                                border = BorderStroke(1.dp, CardBorder),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text(if (vm.state == "Running") "Stop" else "Start", color = AccentCyan)
                            }
                            Button(
                                onClick = { onOpenVmDisplay(vm.vnc_port) },
                                colors = ButtonDefaults.buttonColors(containerColor = AccentPurple),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text("Display", color = TextPrimary)
                            }
                            Button(
                                onClick = {
                                    scope.launch {
                                        NativeBridge.deleteVm(vm.id)
                                            .onSuccess { statusMessage = "Deleted '${vm.id}'" }
                                            .onFailure { statusMessage = it.message }
                                        refreshVms()
                                    }
                                },
                                colors = ButtonDefaults.buttonColors(containerColor = StatusRed.copy(alpha = 0.2f)),
                                shape = RoundedCornerShape(8.dp)
                            ) {
                                Text("Delete", color = StatusRed)
                            }
                        }
                    }
                }
            }
        }
    }
}
