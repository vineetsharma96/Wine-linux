package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.SystemDiagnosticsData
import com.androidpc.environment.ui.theme.*

@Composable
fun SystemInfoScreen(
    diagnostics: SystemDiagnosticsData?,
    onBack: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark)
            .padding(16.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            TextButton(onClick = onBack) {
                Text("← Back", color = AccentCyan)
            }
            Spacer(modifier = Modifier.width(8.dp))
            Text(
                text = "System Diagnostics",
                style = MaterialTheme.typography.titleLarge,
                color = TextPrimary,
                fontWeight = FontWeight.Bold
            )
        }

        Spacer(modifier = Modifier.height(16.dp))

        LazyColumn(
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxSize()
        ) {
            item {
                DiagCard("Engine Architecture", diagnostics?.host_arch ?: "aarch64")
            }
            item {
                DiagCard("Engine Version", diagnostics?.engine_version ?: "0.1.0-alpha.0")
            }
            item {
                DiagCard("Operating System Target", diagnostics?.target_os ?: "android")
            }
            item {
                DiagCard("Rootless PTrace Supported", if (diagnostics?.rootless_ptrace_supported == true) "Yes (Verified)" else "Simulated")
            }
            item {
                DiagCard("Seccomp Restricted", if (diagnostics?.seccomp_restricted == true) "Active (App Sandbox)" else "Unrestricted")
            }
            item {
                DiagCard("PRoot Compatibility", if (diagnostics?.proot_ready == true) "Ready for Phase 1 Rootfs" else "Pending")
            }
            item {
                DiagCard("Active Processes", "${diagnostics?.active_processes_count ?: 0}")
            }
        }
    }
}

@Composable
fun DiagCard(label: String, value: String) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
        colors = CardDefaults.cardColors(containerColor = SurfaceDark),
        shape = RoundedCornerShape(10.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(14.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(text = label, style = MaterialTheme.typography.bodyMedium, color = TextSecondary)
            Text(text = value, style = MaterialTheme.typography.bodyMedium, color = TextPrimary, fontWeight = FontWeight.SemiBold)
        }
    }
}
