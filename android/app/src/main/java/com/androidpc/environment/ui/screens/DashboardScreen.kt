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
import androidx.compose.ui.unit.sp
import androidx.compose.ui.graphics.Color
import com.androidpc.environment.bridge.EngineStatusData
import com.androidpc.environment.bridge.HealthReportData
import com.androidpc.environment.ui.theme.*

@Composable
fun DashboardScreen(
    engineStatus: EngineStatusData?,
    healthReport: HealthReportData?,
    isRefreshing: Boolean,
    onRefresh: () -> Unit,
    onCreateEnvironment: () -> Unit,
    onOpenHealthCheck: () -> Unit,
    onOpenSystemInfo: () -> Unit,
    onOpenDisplayViewport: () -> Unit = {}
) {
    LazyColumn(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        item {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        text = "Android PC",
                        style = MaterialTheme.typography.titleMedium,
                        color = AccentCyan,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = "Dashboard",
                        style = MaterialTheme.typography.headlineLarge,
                        color = TextPrimary,
                        fontWeight = FontWeight.Bold
                    )
                }

                Button(
                    onClick = onRefresh,
                    enabled = !isRefreshing,
                    colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                    border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                    shape = RoundedCornerShape(8.dp)
                ) {
                    Text(
                        text = if (isRefreshing) "Refreshing..." else "Poll Rust",
                        color = TextPrimary,
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
            }
        }

        // Engine Status Card
        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "CORE ENGINE STATUS",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted,
                            letterSpacing = 1.sp
                        )

                        val stateColor = when (engineStatus?.state) {
                            "Ready" -> StatusGreen
                            "Degraded" -> StatusAmber
                            else -> AccentCyan
                        }

                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Box(
                                modifier = Modifier
                                    .size(8.dp)
                                    .background(stateColor, RoundedCornerShape(4.dp))
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = engineStatus?.state ?: "Connecting...",
                                style = MaterialTheme.typography.bodyMedium,
                                color = stateColor,
                                fontWeight = FontWeight.Bold
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(14.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        MetricItem("Version", engineStatus?.version ?: "0.1.0-alpha.0")
                        MetricItem("Arch", engineStatus?.host_arch ?: "aarch64")
                        MetricItem("Profile", engineStatus?.build_profile ?: "release")
                        MetricItem("Uptime", "${engineStatus?.uptime_seconds ?: 0}s")
                    }
                }
            }
        }

        // Health Status Summary Card
        item {
            Card(
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, CardBorder, RoundedCornerShape(14.dp)),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                shape = RoundedCornerShape(14.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "SYSTEM HEALTH REPORT",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted,
                            letterSpacing = 1.sp
                        )

                        val statusText = healthReport?.status ?: "Healthy"
                        val statusColor = when (statusText) {
                            "Healthy" -> StatusGreen
                            "Degraded" -> StatusAmber
                            else -> StatusRed
                        }

                        Text(
                            text = statusText,
                            style = MaterialTheme.typography.bodyMedium,
                            color = statusColor,
                            fontWeight = FontWeight.Bold
                        )
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    healthReport?.subsystems?.take(3)?.forEach { sub ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 4.dp),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = sub.name,
                                style = MaterialTheme.typography.bodyMedium,
                                color = TextSecondary
                            )
                            Text(
                                text = "${sub.latency_ms}ms",
                                style = MaterialTheme.typography.labelSmall,
                                color = TextMuted
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    TextButton(
                        onClick = onOpenHealthCheck,
                        modifier = Modifier.align(Alignment.End)
                    ) {
                        Text("View Subsystem Details →", color = AccentCyan)
                    }
                }
            }
        }

        // Windows Compatibility (Wine + Box64)
        item {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = SurfaceDark),
                border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                shape = RoundedCornerShape(12.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "WINDOWS COMPATIBILITY (WINE + BOX64)",
                            style = MaterialTheme.typography.labelSmall,
                            color = TextMuted,
                            letterSpacing = 1.sp
                        )
                        Surface(
                            shape = RoundedCornerShape(4.dp),
                            color = StatusGreen.copy(alpha = 0.15f)
                        ) {
                            Text(
                                text = "READY",
                                color = StatusGreen,
                                style = MaterialTheme.typography.labelSmall,
                                fontWeight = FontWeight.Bold,
                                modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        MetricItem(label = "Wine Architecture", value = "Win64 / WoW64")
                        MetricItem(label = "Box64 Dynarec", value = "Active (ARM64)")
                    }

                    Spacer(modifier = Modifier.height(12.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        MetricItem(label = "Default Prefix", value = "~/.wine (Drive C:)")
                        MetricItem(label = "Target Platform", value = "Windows 10 x64")
                    }

                    Spacer(modifier = Modifier.height(14.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Button(
                            onClick = { /* Wineboot init action */ },
                            modifier = Modifier
                                .weight(1f)
                                .height(38.dp),
                            colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                            border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                            shape = RoundedCornerShape(8.dp)
                        ) {
                            Text("Init C: Drive", style = MaterialTheme.typography.labelMedium, color = AccentCyan)
                        }

                        Button(
                            onClick = { /* Launch Windows App */ },
                            modifier = Modifier
                                .weight(1f)
                                .height(38.dp),
                            colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                            border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                            shape = RoundedCornerShape(8.dp)
                        ) {
                            Text("Run .exe", style = MaterialTheme.typography.labelMedium, color = TextPrimary)
                        }
                    }
                }
            }
        }

        // Quick Actions
        item {
            Text(
                text = "ACTIONS",
                style = MaterialTheme.typography.labelSmall,
                color = TextMuted,
                letterSpacing = 1.sp
            )

            Spacer(modifier = Modifier.height(8.dp))

            Button(
                onClick = onOpenDisplayViewport,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp),
                colors = ButtonDefaults.buttonColors(containerColor = AccentPurple),
                shape = RoundedCornerShape(10.dp)
            ) {
                Text(
                    text = "🖥  Launch Desktop Viewport (Xvnc)",
                    color = Color.White,
                    fontWeight = FontWeight.Bold
                )
            }

            Spacer(modifier = Modifier.height(10.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Button(
                    onClick = onCreateEnvironment,
                    modifier = Modifier
                        .weight(1f)
                        .height(48.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        text = "+ New Environment",
                        color = BgDark,
                        fontWeight = FontWeight.Bold
                    )
                }

                Button(
                    onClick = onOpenSystemInfo,
                    modifier = Modifier
                        .weight(1f)
                        .height(48.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                    border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder),
                    shape = RoundedCornerShape(10.dp)
                ) {
                    Text(
                        text = "Diagnostics",
                        color = TextPrimary
                    )
                }
            }
        }
    }
}

@Composable
fun MetricItem(label: String, value: String) {
    Column {
        Text(text = label, style = MaterialTheme.typography.labelSmall, color = TextMuted)
        Spacer(modifier = Modifier.height(2.dp))
        Text(
            text = value,
            style = MaterialTheme.typography.bodyLarge,
            color = TextPrimary,
            fontWeight = FontWeight.SemiBold
        )
    }
}
