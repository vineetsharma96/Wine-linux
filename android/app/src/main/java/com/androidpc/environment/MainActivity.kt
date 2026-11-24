package com.androidpc.environment

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.lifecycle.lifecycleScope
import com.androidpc.environment.bridge.EngineStatusData
import com.androidpc.environment.bridge.HealthReportData
import com.androidpc.environment.bridge.NativeBridge
import com.androidpc.environment.bridge.SystemDiagnosticsData
import com.androidpc.environment.ui.screens.*
import com.androidpc.environment.ui.theme.AndroidPCEnvironmentTheme
import com.androidpc.environment.ui.theme.BgDark
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    enum class Screen {
        Splash,
        Onboarding,
        Dashboard,
        HealthCheck,
        SystemInfo,
        CreateEnvironment,
        DisplayViewport,
        Settings
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Initialize Rust Engine in background IO thread
        lifecycleScope.launch {
            NativeBridge.initEngine(applicationContext)
        }

        setContent {
            AndroidPCEnvironmentTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = BgDark
                ) {
                    var currentScreen by remember { mutableStateOf(Screen.Splash) }
                    var engineStatus by remember { mutableStateOf<EngineStatusData?>(null) }
                    var healthReport by remember { mutableStateOf<HealthReportData?>(null) }
                    var diagnostics by remember { mutableStateOf<SystemDiagnosticsData?>(null) }
                    var isRefreshing by remember { mutableStateOf(false) }

                    val refreshData = {
                        isRefreshing = true
                        lifecycleScope.launch {
                            val statusRes = NativeBridge.getEngineStatus()
                            statusRes.onSuccess { engineStatus = it }

                            val healthRes = NativeBridge.runHealthCheck()
                            healthRes.onSuccess { healthReport = it }

                            val diagRes = NativeBridge.getDiagnostics()
                            diagRes.onSuccess { diagnostics = it }

                            isRefreshing = false
                        }
                    }

                    LaunchedEffect(Unit) {
                        refreshData()
                    }

                    when (currentScreen) {
                        Screen.Splash -> SplashScreen {
                            currentScreen = Screen.Dashboard
                        }
                        Screen.Onboarding -> OnboardingScreen {
                            currentScreen = Screen.Dashboard
                        }
                        Screen.Dashboard -> DashboardScreen(
                            engineStatus = engineStatus,
                            healthReport = healthReport,
                            isRefreshing = isRefreshing,
                            onRefresh = { refreshData() },
                            onCreateEnvironment = { currentScreen = Screen.CreateEnvironment },
                            onOpenHealthCheck = { currentScreen = Screen.HealthCheck },
                            onOpenSystemInfo = { currentScreen = Screen.SystemInfo },
                            onOpenDisplayViewport = { currentScreen = Screen.DisplayViewport }
                        )
                        Screen.DisplayViewport -> DisplayViewportScreen(
                            onDisconnect = { currentScreen = Screen.Dashboard }
                        )
                        Screen.HealthCheck -> HealthStatusScreen(
                            healthReport = healthReport,
                            onBack = { currentScreen = Screen.Dashboard },
                            onRunCheck = { refreshData() }
                        )
                        Screen.SystemInfo -> SystemInfoScreen(
                            diagnostics = diagnostics,
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                        Screen.CreateEnvironment -> CreateEnvironmentScreen(
                            onBack = { currentScreen = Screen.Dashboard },
                            onCreated = { distro ->
                                refreshData()
                                currentScreen = Screen.DisplayViewport
                            }
                        )
                        Screen.Settings -> SettingsScreen(
                            onBack = { currentScreen = Screen.Dashboard }
                        )
                    }
                }
            }
        }
    }
}
