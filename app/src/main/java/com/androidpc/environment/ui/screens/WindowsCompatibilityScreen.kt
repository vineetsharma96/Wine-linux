package com.androidpc.environment.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.*
import com.androidpc.environment.ui.theme.*
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WindowsCompatibilityScreen(
    environments: List<EnvironmentConfigData>,
    selectedEnvId: String,
    initialTab: Int = 0,
    onSelectEnv: (String) -> Unit,
    onOpenBox64: () -> Unit,
    onBack: () -> Unit
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var selectedTab by remember(initialTab) { mutableIntStateOf(initialTab.coerceIn(0, 3)) }
    var overview by remember { mutableStateOf(WindowsCompatibilityOverviewData(environment_id = selectedEnvId)) }
    var prefixes by remember { mutableStateOf<List<WinePrefixMetadataData>>(emptyList()) }
    var apps by remember { mutableStateOf<List<WindowsAppProfileData>>(emptyList()) }
    var processes by remember { mutableStateOf<List<TrackedWindowsProcessData>>(emptyList()) }
    var dependencies by remember { mutableStateOf<List<DependencyProfileData>>(emptyList()) }
    var diagnosis by remember { mutableStateOf<PrefixDependencyDiagnosisData?>(null) }
    var statusMessage by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(false) }

    val refreshAll: () -> Unit = {
        scope.launch {
            isLoading = true
            overview = WindowsCompatibilityBridge.getOverview(context, selectedEnvId)
            prefixes = WindowsCompatibilityBridge.listWinePrefixes(context)
            apps = WindowsCompatibilityBridge.listWindowsApps(context)
            processes = WindowsCompatibilityBridge.listWindowsProcesses()
            dependencies = WindowsCompatibilityBridge.listDependencyProfiles()
            isLoading = false
        }
    }

    LaunchedEffect(selectedEnvId) {
        refreshAll()
    }

    val tabs = listOf("Apps", "Wine & Prefixes", "Graphics & DX", "Deps & Diag")

    Scaffold(
        containerColor = BgDark,
        topBar = {
            TopAppBar(
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = SurfaceDark,
                    titleContentColor = TextPrimary
                ),
                navigationIcon = {
                    IconButton(
                        onClick = onBack,
                        modifier = Modifier.testTag("windows_compat_back_button")
                    ) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back to Containers", tint = TextPrimary)
                    }
                },
                title = {
                    Column {
                        Text("Windows Compatibility", style = MaterialTheme.typography.titleLarge, color = TextPrimary)
                        Text(
                            text = "Box64 ${overview.box64.pinned_version} • Wine ${overview.wine.pinned_version} • ${overview.graphics.effective_backend.uppercase()}",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextSecondary
                        )
                    }
                },
                actions = {
                    IconButton(onClick = refreshAll, enabled = !isLoading) {
                        Icon(
                            Icons.Filled.Refresh,
                            contentDescription = "Refresh Windows compatibility status",
                            tint = if (isLoading) TextMuted else AccentCyan
                        )
                    }
                }
            )
        }
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
        ) {
            // Environment selector pills if multiple containers exist
            if (environments.isNotEmpty()) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    environments.take(3).forEach { env ->
                        FilterChip(
                            selected = env.id == selectedEnvId,
                            onClick = { onSelectEnv(env.id) },
                            label = { Text(env.display_name ?: env.id, style = MaterialTheme.typography.labelSmall) }
                        )
                    }
                }
            }

            SecondaryScrollableTabRow(
                selectedTabIndex = selectedTab,
                containerColor = SurfaceDark,
                contentColor = AccentCyan,
                edgePadding = 12.dp
            ) {
                tabs.forEachIndexed { idx, title ->
                    Tab(
                        selected = selectedTab == idx,
                        onClick = { selectedTab = idx },
                        modifier = Modifier.testTag("windows_compat_tab_$idx"),
                        text = {
                            Text(
                                text = title,
                                color = if (selectedTab == idx) AccentCyan else TextSecondary,
                                style = MaterialTheme.typography.labelMedium
                            )
                        }
                    )
                }
            }

            statusMessage?.let { msg ->
                Surface(
                    color = SurfaceContainer,
                    shape = RoundedCornerShape(8.dp),
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp)
                ) {
                    Text(
                        text = msg,
                        color = AccentCyan,
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.padding(10.dp)
                    )
                }
            }

            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(horizontal = 16.dp),
                contentPadding = PaddingValues(top = 12.dp, bottom = 32.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                item {
                    when (selectedTab) {
                        0 -> WindowsAppsTabContent(
                            selectedEnvId = selectedEnvId,
                            apps = apps,
                            prefixes = prefixes,
                            processes = processes,
                            onRegisterApp = { profile ->
                                scope.launch {
                                    WindowsCompatibilityBridge.registerWindowsApp(context, profile)
                                        .onSuccess { statusMessage = "Saved Windows application '${it.name}'" }
                                        .onFailure { statusMessage = "Save failed: ${it.message}" }
                                    refreshAll()
                                }
                            },
                            onLaunchApp = { app ->
                                scope.launch {
                                    WindowsCompatibilityBridge.launchWindowsApp(context, app)
                                        .onSuccess { res ->
                                            val warn = res.warnings.firstOrNull()?.let { " — $it" } ?: ""
                                            statusMessage = "Launch prepared for ${app.name} (${res.effective_graphics_backend.uppercase()}, ${res.dx_choice})$warn"
                                        }
                                        .onFailure { statusMessage = "Launch error: ${it.message}" }
                                    refreshAll()
                                }
                            },
                            onStopApp = { appId, force ->
                                scope.launch {
                                    WindowsCompatibilityBridge.terminateWindowsApp(appId, force)
                                        .onSuccess { statusMessage = "Terminated '$appId' in its isolated prefix" }
                                        .onFailure { statusMessage = it.message }
                                    refreshAll()
                                }
                            },
                            onRemoveApp = { appId ->
                                scope.launch {
                                    WindowsCompatibilityBridge.removeWindowsApp(context, appId)
                                        .onSuccess { statusMessage = "Removed application '$appId'" }
                                        .onFailure { statusMessage = it.message }
                                    refreshAll()
                                }
                            }
                        )
                        1 -> WineAndPrefixesTabContent(
                            selectedEnvId = selectedEnvId,
                            wineStatus = overview.wine,
                            prefixes = prefixes,
                            processes = processes,
                            onCreatePrefix = { id, name, winVer, gfx ->
                                scope.launch {
                                    WindowsCompatibilityBridge.createWinePrefix(
                                        context = context,
                                        id = id,
                                        displayName = name,
                                        envId = selectedEnvId,
                                        windowsVersion = winVer,
                                        graphicsBackend = gfx
                                    ).onSuccess { statusMessage = "Created isolated prefix '${it.display_name}'" }
                                        .onFailure { statusMessage = "Prefix error: ${it.message}" }
                                    refreshAll()
                                }
                            },
                            onPrefixAction = { action, prefixId ->
                                scope.launch {
                                    val payload = if (action == "clone") {
                                        mapOf("new_id" to "${prefixId}-copy", "new_name" to "$prefixId Copy")
                                    } else emptyMap()
                                    WindowsCompatibilityBridge.manageWinePrefix(context, action, prefixId, payload)
                                        .onSuccess { statusMessage = "Prefix $action ($prefixId): OK" }
                                        .onFailure { statusMessage = "Prefix $action error: ${it.message}" }
                                    refreshAll()
                                }
                            },
                            onStopProcess = { appId, force ->
                                scope.launch {
                                    WindowsCompatibilityBridge.terminateWindowsApp(appId, force)
                                        .onSuccess { statusMessage = "Stopped process '$appId'" }
                                        .onFailure { statusMessage = it.message }
                                    refreshAll()
                                }
                            }
                        )
                        2 -> GraphicsAndDxTabContent(
                            graphics = overview.graphics,
                            onUpdateConfig = { newCfg ->
                                scope.launch {
                                    overview = overview.copy(
                                        graphics = WindowsCompatibilityBridge.configureGraphicsBackend(context, selectedEnvId, newCfg)
                                    )
                                    statusMessage = "Graphics preference set to ${newCfg.preferred_backend.uppercase()} (effective: ${overview.graphics.effective_backend.uppercase()})"
                                }
                            }
                        )
                        else -> DependenciesAndDiagnosticsTabContent(
                            overview = overview,
                            dependencies = dependencies,
                            diagnosis = diagnosis,
                            onRunDiagnosis = {
                                scope.launch {
                                    val targetPrefix = prefixes.firstOrNull()?.id ?: "default-prefix"
                                    diagnosis = WindowsCompatibilityBridge.diagnosePrefixDependencies(targetPrefix)
                                    statusMessage = "Completed DLL & conflict scan for prefix '$targetPrefix'"
                                }
                            },
                            onOpenBox64 = onOpenBox64
                        )
                    }
                }
            }
        }
    }
}
