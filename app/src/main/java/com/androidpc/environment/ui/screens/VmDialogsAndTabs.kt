package com.androidpc.environment.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.*
import com.androidpc.environment.ui.theme.*

@Composable
internal fun CreateVmDialog(
    onDismiss: () -> Unit,
    onCreate: (
        id: String,
        name: String,
        arch: String,
        memoryMb: Int,
        cpus: Int,
        diskSizeMb: Long,
        diskFormat: String,
        isoPath: String?,
        networkMode: String,
        audioBackend: String,
        displayBackend: String,
        firmwareKind: String
    ) -> Unit
) {
    var vmId by remember { mutableStateOf("alpine-arm64") }
    var vmName by remember { mutableStateOf("Alpine ARM64 VM") }
    var arch by remember { mutableStateOf("aarch64") }
    var memoryMb by remember { mutableFloatStateOf(1024f) }
    var cpus by remember { mutableFloatStateOf(2f) }
    var diskMb by remember { mutableFloatStateOf(2048f) }
    var diskFormat by remember { mutableStateOf("qcow2") }
    var isoPath by remember { mutableStateOf("") }

    // Advanced hardware settings separated from ordinary VM controls
    var showAdvanced by remember { mutableStateOf(false) }
    var networkMode by remember { mutableStateOf("user_slirp") }
    var audioBackend by remember { mutableStateOf("disabled") }
    var displayBackend by remember { mutableStateOf("vnc_localhost") }
    var firmwareKind by remember { mutableStateOf("aavmf_uefi") }

    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = SurfaceDark,
        title = { Text("Create QEMU Virtual Machine", color = TextPrimary, fontWeight = FontWeight.Bold) },
        text = {
            Column(
                modifier = Modifier.verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                OutlinedTextField(
                    value = vmId,
                    onValueChange = { vmId = it },
                    label = { Text("VM ID (alphanumeric, -, _)", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("vm_create_id_input"),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary,
                        focusedBorderColor = AccentCyan,
                        unfocusedBorderColor = CardBorder
                    )
                )
                OutlinedTextField(
                    value = vmName,
                    onValueChange = { vmName = it },
                    label = { Text("Display Name", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("vm_create_name_input"),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary,
                        focusedBorderColor = AccentCyan,
                        unfocusedBorderColor = CardBorder
                    )
                )
                Text("Guest Architecture", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(
                        selected = arch == "aarch64",
                        onClick = {
                            arch = "aarch64"
                            firmwareKind = "aavmf_uefi"
                        },
                        label = { Text("ARM64 (virt)") },
                        modifier = Modifier.testTag("vm_arch_arm64_chip")
                    )
                    FilterChip(
                        selected = arch == "x86_64",
                        onClick = {
                            arch = "x86_64"
                            firmwareKind = "ovmf_uefi"
                        },
                        label = { Text("x86-64 (TCG Emulated)") },
                        modifier = Modifier.testTag("vm_arch_x86_chip")
                    )
                }
                if (arch == "x86_64") {
                    Text(
                        "x86-64 guests run via QEMU TCG software emulation on ARM64 Android — expect significantly slower execution than native ARM64.",
                        style = MaterialTheme.typography.labelSmall,
                        color = StatusAmber
                    )
                }

                Text("RAM: ${memoryMb.toInt()} MB", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                Slider(value = memoryMb, onValueChange = { memoryMb = it }, valueRange = 512f..6144f, steps = 10)

                Text("vCPUs: ${cpus.toInt()}  •  Disk: ${diskMb.toInt()} MB", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                Slider(value = cpus, onValueChange = { cpus = it }, valueRange = 1f..8f, steps = 6)
                Slider(value = diskMb, onValueChange = { diskMb = it }, valueRange = 512f..16384f, steps = 14)

                Text("Virtual Disk Format", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(selected = diskFormat == "qcow2", onClick = { diskFormat = "qcow2" }, label = { Text("QCOW2 (Snapshots)") })
                    FilterChip(selected = diskFormat == "raw", onClick = { diskFormat = "raw" }, label = { Text("RAW (Sparse)") })
                }

                OutlinedTextField(
                    value = isoPath,
                    onValueChange = { isoPath = it },
                    label = { Text("Optional Boot ISO Path", color = TextSecondary) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = TextPrimary,
                        unfocusedTextColor = TextPrimary,
                        focusedBorderColor = AccentCyan,
                        unfocusedBorderColor = CardBorder
                    )
                )

                TextButton(
                    onClick = { showAdvanced = !showAdvanced },
                    modifier = Modifier.testTag("vm_toggle_advanced_settings")
                ) {
                    Icon(
                        imageVector = if (showAdvanced) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                        contentDescription = "Toggle advanced VM settings",
                        tint = AccentCyan
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        if (showAdvanced) "Hide Advanced Hardware Settings" else "Show Advanced Hardware Settings",
                        color = AccentCyan
                    )
                }

                if (showAdvanced) {
                    HorizontalDivider(color = CardBorder)
                    Text("Firmware", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        val options = if (arch == "aarch64") {
                            listOf("aavmf_uefi" to "AAVMF UEFI", "direct_kernel" to "Direct Kernel")
                        } else {
                            listOf("ovmf_uefi" to "OVMF UEFI", "sea_bios" to "SeaBIOS")
                        }
                        options.forEach { (key, label) ->
                            FilterChip(selected = firmwareKind == key, onClick = { firmwareKind = key }, label = { Text(label) })
                        }
                    }

                    Text("Network Mode (Rootless SLIRP)", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        FilterChip(selected = networkMode == "user_slirp", onClick = { networkMode = "user_slirp" }, label = { Text("SLIRP NAT") })
                        FilterChip(selected = networkMode == "isolated_slirp", onClick = { networkMode = "isolated_slirp" }, label = { Text("Isolated") })
                        FilterChip(selected = networkMode == "disabled", onClick = { networkMode = "disabled" }, label = { Text("Off") })
                    }

                    Text("Display & Audio", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        FilterChip(selected = displayBackend == "vnc_localhost", onClick = { displayBackend = "vnc_localhost" }, label = { Text("VNC Display") })
                        FilterChip(selected = displayBackend == "headless_serial", onClick = { displayBackend = "headless_serial" }, label = { Text("Headless") })
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        FilterChip(selected = audioBackend == "disabled", onClick = { audioBackend = "disabled" }, label = { Text("Audio Off") })
                        FilterChip(selected = audioBackend == "pulse_loopback", onClick = { audioBackend = "pulse_loopback" }, label = { Text("Pulse Loopback") })
                    }
                }
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    onCreate(
                        vmId.trim(),
                        vmName.trim(),
                        arch,
                        memoryMb.toInt(),
                        cpus.toInt(),
                        diskMb.toLong(),
                        diskFormat,
                        isoPath.trim().takeIf { it.isNotEmpty() },
                        networkMode,
                        audioBackend,
                        displayBackend,
                        firmwareKind
                    )
                },
                colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                modifier = Modifier.heightIn(min = 48.dp).testTag("vm_create_confirm_button")
            ) {
                Text("Create VM", color = BgDark, fontWeight = FontWeight.Bold)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, modifier = Modifier.heightIn(min = 48.dp)) {
                Text("Cancel", color = TextSecondary)
            }
        }
    )
}

@Composable
internal fun ConfirmDeleteVmDialog(
    vmName: String,
    vmId: String,
    onDismiss: () -> Unit,
    onConfirm: () -> Unit
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = SurfaceDark,
        title = { Text("Delete Virtual Machine?", color = StatusRed, fontWeight = FontWeight.Bold) },
        text = {
            Text(
                "Deleting '$vmName' ($vmId) permanently removes its virtual disks, snapshots, and UEFI NVRAM files from app storage. This action cannot be undone.",
                color = TextSecondary,
                style = MaterialTheme.typography.bodyMedium
            )
        },
        confirmButton = {
            Button(
                onClick = onConfirm,
                colors = ButtonDefaults.buttonColors(containerColor = StatusRed),
                modifier = Modifier.heightIn(min = 48.dp).testTag("vm_delete_confirm_button")
            ) {
                Text("Delete Permanently", color = TextPrimary, fontWeight = FontWeight.Bold)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, modifier = Modifier.heightIn(min = 48.dp)) {
                Text("Cancel", color = TextSecondary)
            }
        }
    )
}

@Composable
internal fun VmStorageAndSnapshotsSection(
    vms: List<VmRuntimeSummaryData>,
    selectedVmId: String?,
    onSelectVm: (String) -> Unit,
    diskReports: List<VmDiskInspectionData>,
    snapshots: List<VmSnapshotMetadataData>,
    onAttachDisk: (vmId: String, diskId: String, format: String, sizeMb: Long) -> Unit,
    onCreateSnapshot: (vmId: String, snapId: String, desc: String) -> Unit,
    onRestoreSnapshot: (vmId: String, snapId: String) -> Unit,
    onDeleteSnapshot: (vmId: String, snapId: String) -> Unit
) {
    var snapNameInput by remember { mutableStateOf("clean-base") }
    var extraDiskName by remember { mutableStateOf("data1") }

    if (vms.isEmpty()) {
        Card(
            modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
            colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
            shape = RoundedCornerShape(12.dp)
        ) {
            Text(
                "Create a VM first to inspect virtual QCOW2/RAW disks and manage verified snapshots.",
                modifier = Modifier.padding(16.dp),
                color = TextSecondary,
                style = MaterialTheme.typography.bodySmall
            )
        }
        return
    }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            vms.forEach { vm ->
                FilterChip(
                    selected = vm.profile.id == selectedVmId,
                    onClick = { onSelectVm(vm.profile.id) },
                    label = { Text(vm.profile.id) }
                )
            }
        }

        val activeId = selectedVmId ?: vms.first().profile.id

        Card(
            modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
            colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("VIRTUAL DISKS ($activeId)", style = MaterialTheme.typography.labelSmall, color = AccentCyan, fontWeight = FontWeight.Bold)
                diskReports.forEach { disk ->
                    Column(modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(8.dp)).padding(10.dp)) {
                        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text(disk.path.substringAfterLast('/'), color = TextPrimary, fontWeight = FontWeight.SemiBold)
                            Text(
                                if (disk.healthy) "${disk.detected_format.uppercase()} OK" else "CHECK FAILED",
                                color = if (disk.healthy) StatusGreen else StatusRed,
                                style = MaterialTheme.typography.labelSmall,
                                fontWeight = FontWeight.Bold
                            )
                        }
                        Text(
                            "Virtual: ${disk.virtual_size_bytes / (1024 * 1024)} MB • On-disk: ${disk.actual_disk_bytes / 1024} KiB",
                            style = MaterialTheme.typography.bodySmall,
                            color = TextSecondary
                        )
                        disk.diagnostics.firstOrNull()?.let {
                            Text(it, style = MaterialTheme.typography.labelSmall, color = TextMuted)
                        }
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    OutlinedTextField(
                        value = extraDiskName,
                        onValueChange = { extraDiskName = it },
                        label = { Text("New Disk ID", color = TextSecondary) },
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = AccentCyan,
                            unfocusedBorderColor = CardBorder
                        )
                    )
                    Button(
                        onClick = { onAttachDisk(activeId, extraDiskName.trim().ifEmpty { "data1" }, "qcow2", 1024L) },
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                        border = BorderStroke(1.dp, CardBorder),
                        modifier = Modifier.heightIn(min = 48.dp).testTag("vm_attach_disk_button")
                    ) {
                        Text("+1 GB QCOW2", color = AccentCyan)
                    }
                }
            }
        }

        Card(
            modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
            colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("SNAPSHOTS ($activeId)", style = MaterialTheme.typography.labelSmall, color = AccentCyan, fontWeight = FontWeight.Bold)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    OutlinedTextField(
                        value = snapNameInput,
                        onValueChange = { snapNameInput = it },
                        label = { Text("Snapshot ID", color = TextSecondary) },
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                        colors = OutlinedTextFieldDefaults.colors(
                            focusedTextColor = TextPrimary,
                            unfocusedTextColor = TextPrimary,
                            focusedBorderColor = AccentCyan,
                            unfocusedBorderColor = CardBorder
                        )
                    )
                    Button(
                        onClick = { onCreateSnapshot(activeId, snapNameInput.trim().ifEmpty { "snap-1" }, "Manual user snapshot") },
                        colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                        modifier = Modifier.heightIn(min = 48.dp).testTag("vm_create_snapshot_button")
                    ) {
                        Text("Snapshot", color = BgDark, fontWeight = FontWeight.Bold)
                    }
                }
                if (snapshots.isEmpty()) {
                    Text("No snapshots recorded yet.", style = MaterialTheme.typography.bodySmall, color = TextMuted)
                } else {
                    snapshots.forEach { snap ->
                        Row(
                            modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(8.dp)).padding(10.dp),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(snap.name, color = TextPrimary, fontWeight = FontWeight.SemiBold)
                                Text(
                                    "${snap.mechanism} • ${snap.disk_sha256?.take(12) ?: "internal qcow2"}",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = TextSecondary
                                )
                            }
                            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                                TextButton(onClick = { onRestoreSnapshot(activeId, snap.snapshot_id) }) {
                                    Text("Restore", color = AccentCyan)
                                }
                                TextButton(onClick = { onDeleteSnapshot(activeId, snap.snapshot_id) }) {
                                    Text("Delete", color = StatusRed)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
internal fun VmOfficialImagesSection(images: List<VmGuestImageEntryData>) {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Card(
            modifier = Modifier.fillMaxWidth().border(1.dp, AccentPurple.copy(alpha = 0.5f), RoundedCornerShape(12.dp)),
            colors = CardDefaults.cardColors(containerColor = SurfaceElevated),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text("OFFICIAL GUEST IMAGE POLICY", style = MaterialTheme.typography.labelSmall, color = AccentCyan, fontWeight = FontWeight.Bold)
                Text(
                    "Android PC Environment never bundles copyrighted OS images or proprietary firmware. Download verified images directly from official vendor HTTPS portals and verify SHA-256 before booting.",
                    style = MaterialTheme.typography.bodySmall,
                    color = TextSecondary
                )
            }
        }

        images.forEach { img ->
            Card(
                modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        Text(img.display_name, color = TextPrimary, fontWeight = FontWeight.Bold, modifier = Modifier.weight(1f))
                        Text(
                            img.arch.uppercase(),
                            style = MaterialTheme.typography.labelSmall,
                            color = if (img.arch == "aarch64") AccentCyan else StatusAmber,
                            fontWeight = FontWeight.Bold
                        )
                    }
                    Text("Version ${img.version} • Min ${img.min_ram_mb} MB RAM • Min ${img.min_disk_mb} MB Disk • ${img.license}", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                    Text(img.notes, style = MaterialTheme.typography.bodySmall, color = TextSecondary)
                    Text("URL: ${img.official_download_url}", style = MaterialTheme.typography.labelSmall, color = AccentCyan, fontFamily = FontFamily.Monospace)
                    Text("SHA-256 Feed: ${img.checksum_url}", style = MaterialTheme.typography.labelSmall, color = TextMuted, fontFamily = FontFamily.Monospace)
                }
            }
        }
    }
}

@Composable
internal fun VmDiagnosticsSection(
    caps: QemuEngineCapabilitiesData?,
    selectedVmId: String?,
    logs: List<String>
) {
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Card(
            modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
            colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("QEMU ENGINE & PLATFORM REALITY", style = MaterialTheme.typography.labelSmall, color = AccentCyan, fontWeight = FontWeight.Bold)
                Text("Pinned QEMU: v${caps?.pinned_qemu_version ?: "11.1.3"} • EDK2: ${caps?.pinned_edk2_version ?: "edk2-stable202608"} • libslirp: v${caps?.pinned_libslirp_version ?: "4.9.7"}", style = MaterialTheme.typography.bodySmall, color = TextPrimary)
                Text("qemu-system-aarch64: ${if (caps?.qemu_aarch64_installed == true) "STAGED" else "PENDING"} • qemu-system-x86_64: ${if (caps?.qemu_x86_64_installed == true) "STAGED" else "PENDING"} • qemu-img: ${if (caps?.qemu_img_installed == true) "STAGED" else "PENDING"}", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                Text("UEFI Firmware — AAVMF: ${if (caps?.aavmf_firmware_installed == true) "STAGED" else "PENDING"} • OVMF: ${if (caps?.ovmf_firmware_installed == true) "STAGED" else "PENDING"}", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                HorizontalDivider(color = CardBorder)
                Text("KVM Probe: ${caps?.kvm?.notes ?: "Probing..."}", style = MaterialTheme.typography.bodySmall, color = if (caps?.kvm?.accessible == true) StatusGreen else StatusAmber)
                Text("AVF Status: ${caps?.avf?.notes ?: ""}", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                Text("Networking: ${caps?.networking_notes ?: ""}", style = MaterialTheme.typography.labelSmall, color = TextSecondary)
                Text("Security Boundary: ${caps?.security_boundary_note ?: ""}", style = MaterialTheme.typography.labelSmall, color = TextMuted)
            }
        }

        Card(
            modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
            colors = CardDefaults.cardColors(containerColor = SurfaceDark),
            shape = RoundedCornerShape(12.dp)
        ) {
            Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(
                    "VM RUNTIME LOGS (${selectedVmId ?: "none selected"})",
                    style = MaterialTheme.typography.labelSmall,
                    color = AccentCyan,
                    fontWeight = FontWeight.Bold
                )
                if (logs.isEmpty()) {
                    Text("No logs available.", style = MaterialTheme.typography.bodySmall, color = TextMuted)
                } else {
                    logs.forEach { line ->
                        Text(line, style = MaterialTheme.typography.labelSmall, color = TextSecondary, fontFamily = FontFamily.Monospace)
                    }
                }
            }
        }
    }
}
