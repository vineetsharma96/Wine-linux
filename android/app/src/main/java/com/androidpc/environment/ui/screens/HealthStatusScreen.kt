package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.androidpc.environment.bridge.HealthReportData
import com.androidpc.environment.bridge.SubsystemHealthData
import com.androidpc.environment.ui.theme.*

@Composable
fun HealthStatusScreen(
    healthReport: HealthReportData?,
    onBack: () -> Unit,
    onRunCheck: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark)
            .padding(16.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            TextButton(onClick = onBack) {
                Text("← Back", color = AccentCyan)
            }
            Text(
                text = "Engine Health",
                style = MaterialTheme.typography.titleLarge,
                color = TextPrimary,
                fontWeight = FontWeight.Bold
            )
            Button(
                onClick = onRunCheck,
                colors = ButtonDefaults.buttonColors(containerColor = SurfaceDark),
                border = androidx.compose.foundation.BorderStroke(1.dp, CardBorder)
            ) {
                Text("Run Check", color = TextPrimary)
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        LazyColumn(
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxSize()
        ) {
            val list = healthReport?.subsystems ?: emptyList()
            if (list.isEmpty()) {
                item {
                    Text("No health data available. Click 'Run Check' to poll Rust.", color = TextSecondary)
                }
            } else {
                items(list) { sub ->
                    SubsystemCard(sub)
                }
            }
        }
    }
}

@Composable
fun SubsystemCard(sub: SubsystemHealthData) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, CardBorder, RoundedCornerShape(10.dp)),
        colors = CardDefaults.cardColors(containerColor = SurfaceDark),
        shape = RoundedCornerShape(10.dp)
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = sub.name,
                    style = MaterialTheme.typography.titleMedium,
                    color = TextPrimary,
                    fontWeight = FontWeight.SemiBold
                )

                val color = when (sub.level) {
                    "Healthy" -> StatusGreen
                    "Degraded" -> StatusAmber
                    else -> StatusRed
                }

                Text(
                    text = "${sub.level} (${sub.latency_ms}ms)",
                    style = MaterialTheme.typography.labelSmall,
                    color = color,
                    fontWeight = FontWeight.Bold
                )
            }

            Spacer(modifier = Modifier.height(6.dp))

            Text(
                text = sub.message,
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary
            )
        }
    }
}
