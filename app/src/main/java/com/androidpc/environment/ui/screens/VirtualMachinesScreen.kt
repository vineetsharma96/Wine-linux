package com.androidpc.environment.ui.screens

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.*
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.launch

@Composable
fun VirtualMachinesScreen(
    onOpenVmDisplay: (Int) -> Unit,
    onBack: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var selectedTab by remember { mutableIntStateOf(0) }
    var capabilities by remember { mutableStateOf<QemuEngineCapabilitiesData?>(null) }
    var vms by remember { mutableStateOf<List<VmRuntimeSummaryData>>(emptyList()) }
    var officialImages by remember { mutableStateOf<List<VmGuestImageEntryData>>(emptyList()) }
    var selectedVmId by remember { mutableStateOf<String?>(null) }
    var diskReports by remember { mutableStateOf<List<VmDiskInspectionData>>(emptyList()) }
    var snapshots by remember { mutableStateOf<List<VmSnapshotMetadataData>>(emptyList()) }
    var vmLogs by remember { mutableStateOf<List<String>>(emptyList()) }
    var statusMessage by remember { mutableStateOf<String?>(null) }

    var showCreateDialog by remember { mutableStateOf(false) }
    var pendingDeleteVm by remember { mutableStateOf<VmRuntimeSummaryData?>(null) }

    val refreshVmDetails: (String?) -> Unit = { vmId ->
        if (vmId != null) {
            scope.launch {
                VmEngineBridge.inspectDisks(context, vmId).onSuccess { diskReports = it }
                snapshots = VmEngineBridge.listSnapshots(context, vmId)
                vmLogs = VmEngineBridge.getVmLogs(vmId)
            }
        }
    }

    val refreshAll: () -> Unit = {
        scope.launch {
            capabilities = VmEngineBridge.getCapabilities(context)
            val loaded = VmEngineBridge.listVms(context)
            vms = loaded
            if (officialImages.isEmpty()) {
                officialImages = VmEngineBridge.listOfficialImages()
            }
            val nextSelected = selectedVmId?.takeIf { id -> loaded.any { it.profile.id == id } }
                ?: loaded.firstOrNull()?.profile?.id
            selectedVmId = nextSelected
            refreshVmDetails(nextSelected)
        }
    }

    LaunchedEffect(Unit) {
        refreshAll()
    }

    if (showCreateDialog) {
        CreateVmDialog(
            onDismiss = { showCreateDialog = false },
            onCreate = { id, name, arch, mem, cpus, disk, fmt, iso, net, audio, disp, fw ->
                showCreateDialog = false
                scope.launch {
                    VmEngineBridge.createVm(
                        context = context,
                        id = id,
                        name = name,
                        arch = arch,
                        memoryMb = mem,
                        cpus = cpus,
                        diskSizeMb = disk,
                        diskFormat = fmt,
                        isoPath = iso,
                        networkMode = net,
                        audioBackend = audio,
                        displayBackend = disp,
                        firmwareKind = fw
                    ).onSuccess { created ->
                        statusMessage = "Created VM '${created.profile.name}' (${created.profile.arch}, ${created.profile.disks.firstOrNull()?.format?.uppercase() ?: fmt.uppercase()})"
                        selectedVmId = created.profile.id
                        refreshAll()
                    }.onFailure { err ->
                        statusMessage = "Create VM error: ${err.message}"
                    }
                }
            }
        )
    }

    pendingDeleteVm?.let { target ->
        ConfirmDeleteVmDialog(
            vmName = target.profile.name,
            vmId = target.profile.id,
            onDismiss = { pendingDeleteVm = null },
            onConfirm = {
                val id = target.profile.id
                pendingDeleteVm = null
                scope.launch {
                    VmEngineBridge.lifecycleAction(context, id, "delete")
                        .onSuccess { statusMessage = "Deleted VM '$id' and reclaimed its virtual disks." }
                        .onFailure { statusMessage = "Delete failed: ${it.message}" }
                    refreshAll()
                }
            }
        )
    }

    val tabs = listOf("Machines", "Storage & Snaps", "Images", "Diagnostics")

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
                    IconButton(
                        onClick = onBack,
                        modifier = Modifier.size(48.dp).testTag("vm_back_button")
                    ) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = "Navigate back",
                            tint = AccentCyan
                        )
                    }
                    Column {
                        Text(
                            text = "Virtual Machines",
                            style = MaterialTheme.typography.titleLarge,
                            color = TextPrimary,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "QEMU ${capabilities?.pinned_qemu_version ?: "11.1.3"} • Independent of PRoot/Wine",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextSecondary
                        )
                    }
                }
                Button(
                    onClick = { showCreateDialog = true },
                    colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                    shape = RoundedCornerShape(10.dp),
                    modifier = Modifier.heightIn(min = 48.dp).testTag("vm_new_button")
                ) {
                    Icon(Icons.Default.Add, contentDescription = "Create new virtual machine", tint = BgDark)
                    Spacer(modifier = Modifier.width(4.dp))
                    Text("New VM", color = BgDark, fontWeight = FontWeight.Bold)
                }
            }
        }

        // Honest acceleration & platform reality banner
        item {
            val kvmAccessible = capabilities?.kvm?.accessible == true
            val qemuReady = capabilities?.qemu_aarch64_installed == true || capabilities?.qemu_x86_64_installed == true
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(
                        1.dp,
                        if (kvmAccessible) StatusGreen.copy(alpha = 0.6f) else AccentPurple.copy(alpha = 0.55f),
                        RoundedCornerShape(12.dp)
                    ),
                colors = CardDefaults.cardColors(containerColor = SurfaceElevated),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = if (kvmAccessible) "KVM AVAILABLE (ARM64 ONLY)" else "QEMU TCG MULTI-THREADED EMULATION",
                            style = MaterialTheme.typography.labelSmall,
                            color = if (kvmAccessible) StatusGreen else StatusAmber,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = if (qemuReady) "BINARIES STAGED" else "BINARIES PENDING",
                            style = MaterialTheme.typography.labelSmall,
                            color = if (qemuReady) StatusGreen else StatusAmber,
                            fontWeight = FontWeight.Bold
                        )
                    }
                    Text(
                        text = capabilities?.kvm?.notes ?: "Probing QEMU capabilities and /dev/kvm...",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                    Text(
                        text = "x86-64 guests on ARM64 ALWAYS run under software TCG emulation (never native speed). AVF/pKVM requires @SystemApi and is unavailable to ordinary apps.",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextMuted
                    )
                }
            }
        }

        // Navigation Tabs
        item {
            ScrollableTabRow(
                selectedTabIndex = selectedTab,
                containerColor = SurfaceDark,
                contentColor = AccentCyan,
                edgePadding = 0.dp
            ) {
                tabs.forEachIndexed { index, title ->
                    Tab(
                        selected = selectedTab == index,
                        onClick = {
                            selectedTab = index
                            refreshVmDetails(selectedVmId)
                        },
                        text = {
                            Text(
                                title,
                                color = if (selectedTab == index) AccentCyan else TextSecondary,
                                fontWeight = if (selectedTab == index) FontWeight.Bold else FontWeight.Normal
                            )
                        },
                        modifier = Modifier.testTag("vm_tab_${title.lowercase().replace(" ", "_")}")
                    )
                }
            }
        }

        statusMessage?.let { msg ->
            item {
                Card(
                    modifier = Modifier.fillMaxWidth().border(1.dp, AccentCyan.copy(alpha = 0.4f), RoundedCornerShape(10.dp)),
                    colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(msg, modifier = Modifier.padding(12.dp), style = MaterialTheme.typography.bodySmall, color = AccentCyan)
                }
            }
        }

        when (selectedTab) {
            0 -> {
                if (vms.isEmpty()) {
                    item {
                        Card(
                            modifier = Modifier.fillMaxWidth().border(1.dp, CardBorder, RoundedCornerShape(12.dp)),
                            colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
                            shape = RoundedCornerShape(12.dp)
                        ) {
                            Column(modifier = Modifier.padding(18.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                Text("No Virtual Machines Yet", style = MaterialTheme.typography.titleMedium, color = TextPrimary, fontWeight = FontWeight.Bold)
                                Text(
                                    "Tap 'New VM' to create an isolated ARM64 (virt + AAVMF) or x86-64 (q35 + OVMF/SeaBIOS) QEMU virtual machine with QCOW2 or RAW virtual disks and rootless SLIRP networking.",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = TextSecondary
                                )
                            }
                        }
                    }
                } else {
                    items(vms, key = { it.profile.id }) { summary ->
                        VmCard(
                            summary = summary,
                            onAction = { action ->
                                scope.launch {
                                    val newId = if (action == "clone") "${summary.profile.id}-clone" else null
                                    VmEngineBridge.lifecycleAction(
                                        context = context,
                                        vmId = summary.profile.id,
                                        action = action,
                                        newId = newId,
                                        newName = newId?.let { "${summary.profile.name} (Clone)" }
                                    ).onSuccess {
                                        statusMessage = "VM '${summary.profile.id}': action '$action' completed."
                                    }.onFailure { err ->
                                        statusMessage = "VM '${summary.profile.id}' $action failed: ${err.message}"
                                    }
                                    refreshAll()
                                }
                            },
                            onOpenDisplay = { onOpenVmDisplay(summary.profile.display.vncPort()) },
                            onInspect = {
                                selectedVmId = summary.profile.id
                                selectedTab = 1
                                refreshVmDetails(summary.profile.id)
                            },
                            onRequestDelete = { pendingDeleteVm = summary }
                        )
                    }
                }
            }
            1 -> {
                item {
                    VmStorageAndSnapshotsSection(
                        vms = vms,
                        selectedVmId = selectedVmId,
                        onSelectVm = { id ->
                            selectedVmId = id
                            refreshVmDetails(id)
                        },
                        diskReports = diskReports,
                        snapshots = snapshots,
                        onAttachDisk = { vmId, diskId, format, sizeMb ->
                            scope.launch {
                                VmEngineBridge.attachDisk(context, vmId, diskId, format, sizeMb)
                                    .onSuccess { statusMessage = "Attached $sizeMb MB $format disk '$diskId' to '$vmId'." }
                                    .onFailure { statusMessage = "Attach disk failed: ${it.message}" }
                                refreshAll()
                            }
                        },
                        onCreateSnapshot = { vmId, snapId, desc ->
                            scope.launch {
                                VmEngineBridge.manageSnapshot(context, vmId, "create", snapId, snapId, desc)
                                    .onSuccess { statusMessage = "Created verified snapshot '$snapId' for '$vmId'." }
                                    .onFailure { statusMessage = "Snapshot failed: ${it.message}" }
                                refreshAll()
                            }
                        },
                        onRestoreSnapshot = { vmId, snapId ->
                            scope.launch {
                                VmEngineBridge.manageSnapshot(context, vmId, "restore", snapId)
                                    .onSuccess { statusMessage = "Restored snapshot '$snapId' on '$vmId'." }
                                    .onFailure { statusMessage = "Restore failed: ${it.message}" }
                                refreshAll()
                            }
                        },
                        onDeleteSnapshot = { vmId, snapId ->
                            scope.launch {
                                VmEngineBridge.manageSnapshot(context, vmId, "delete", snapId)
                                    .onSuccess { statusMessage = "Deleted snapshot '$snapId'." }
                                    .onFailure { statusMessage = "Delete snapshot failed: ${it.message}" }
                                refreshAll()
                            }
                        }
                    )
                }
            }
            2 -> {
                item {
                    VmOfficialImagesSection(images = officialImages)
                }
            }
            3 -> {
                item {
                    VmDiagnosticsSection(
                        caps = capabilities,
                        selectedVmId = selectedVmId,
                        logs = vmLogs
                    )
                }
            }
        }
    }
}

@Composable
private fun VmCard(
    summary: VmRuntimeSummaryData,
    onAction: (String) -> Unit,
    onOpenDisplay: () -> Unit,
    onInspect: () -> Unit,
    onRequestDelete: () -> Unit
) {
    val p = summary.profile
    val isRunning = p.state == "Running"
    val isPaused = p.state == "Paused"
    val stateColor = when (p.state) {
        "Running" -> StatusGreen
        "Paused", "Starting", "Stopping" -> StatusAmber
        "Error" -> StatusRed
        else -> TextSecondary
    }
    val primaryDisk = p.disks.firstOrNull()

    Card(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, if (isRunning) StatusGreen.copy(alpha = 0.6f) else CardBorder, RoundedCornerShape(12.dp))
            .testTag("vm_card_${p.id}"),
        colors = CardDefaults.cardColors(containerColor = SurfaceContainer),
        shape = RoundedCornerShape(12.dp)
    ) {
        Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(p.name, style = MaterialTheme.typography.titleMedium, color = TextPrimary, fontWeight = FontWeight.Bold)
                    Text(
                        "${p.id} • ${p.arch.uppercase()} (${p.machine_type})",
                        style = MaterialTheme.typography.labelSmall,
                        color = TextSecondary
                    )
                }
                Text(
                    text = "${p.state.uppercase()} • ${summary.effective_accelerator.uppercase()}",
                    style = MaterialTheme.typography.labelSmall,
                    color = stateColor,
                    fontWeight = FontWeight.Bold
                )
            }

            Text(
                text = "RAM ${p.memory_mb} MB • ${p.cpu.cores} vCPU • ${primaryDisk?.format?.uppercase() ?: "NONE"} ${primaryDisk?.size_mb ?: 0} MB • ${summary.snapshot_count} snaps",
                style = MaterialTheme.typography.bodySmall,
                color = TextSecondary
            )
            Text(
                text = "VNC 127.0.0.1:${p.display.vncPort()} • QMP :${p.qmp_port} • Net: ${p.network.mode} • Audio: ${p.audio.backend}",
                style = MaterialTheme.typography.labelSmall,
                color = TextMuted
            )
            Text(
                text = summary.execution_model_note,
                style = MaterialTheme.typography.labelSmall,
                color = if (p.arch == "x86_64") StatusAmber else TextMuted
            )
            summary.startup_error?.let { err ->
                Text("Last error: $err", style = MaterialTheme.typography.labelSmall, color = StatusRed)
            }

            Spacer(modifier = Modifier.height(4.dp))

            // Primary lifecycle actions
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (!isRunning && !isPaused) {
                    Button(
                        onClick = { onAction("start") },
                        colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                        shape = RoundedCornerShape(8.dp),
                        modifier = Modifier.heightIn(min = 48.dp).testTag("vm_start_${p.id}")
                    ) {
                        Icon(Icons.Default.PlayArrow, contentDescription = "Start VM", tint = BgDark)
                        Spacer(modifier = Modifier.width(4.dp))
                        Text("Start", color = BgDark, fontWeight = FontWeight.Bold)
                    }
                } else {
                    Button(
                        onClick = { onAction(if (isPaused) "resume" else "pause") },
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                        border = BorderStroke(1.dp, StatusAmber),
                        shape = RoundedCornerShape(8.dp),
                        modifier = Modifier.heightIn(min = 48.dp).testTag("vm_pause_resume_${p.id}")
                    ) {
                        Text(if (isPaused) "Resume" else "Pause", color = StatusAmber)
                    }
                    Button(
                        onClick = { onAction("graceful_shutdown") },
                        colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                        border = BorderStroke(1.dp, AccentCyan),
                        shape = RoundedCornerShape(8.dp),
                        modifier = Modifier.heightIn(min = 48.dp).testTag("vm_shutdown_${p.id}")
                    ) {
                        Text("Shutdown", color = AccentCyan)
                    }
                }

                Button(
                    onClick = onOpenDisplay,
                    colors = ButtonDefaults.buttonColors(containerColor = AccentPurple),
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier.heightIn(min = 48.dp).testTag("vm_display_${p.id}")
                ) {
                    Text("Display", color = TextPrimary)
                }
            }

            // Secondary lifecycle & storage actions
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (isRunning || isPaused) {
                    TextButton(onClick = { onAction("restart") }, modifier = Modifier.heightIn(min = 48.dp)) {
                        Text("Restart", color = AccentCyan)
                    }
                    TextButton(onClick = { onAction("force_stop") }, modifier = Modifier.heightIn(min = 48.dp)) {
                        Text("Force Stop", color = StatusRed)
                    }
                } else {
                    TextButton(onClick = { onAction("clone") }, modifier = Modifier.heightIn(min = 48.dp)) {
                        Text("Clone", color = AccentCyan)
                    }
                }
                TextButton(onClick = onInspect, modifier = Modifier.heightIn(min = 48.dp)) {
                    Text("Disks & Snaps", color = TextSecondary)
                }
                Spacer(modifier = Modifier.weight(1f))
                TextButton(
                    onClick = onRequestDelete,
                    modifier = Modifier.heightIn(min = 48.dp).testTag("vm_delete_${p.id}")
                ) {
                    Text("Delete", color = StatusRed)
                }
            }
        }
    }
}
