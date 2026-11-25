package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.androidpc.environment.ui.theme.*

@Composable
fun OnboardingScreen(onGetStarted: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark)
            .padding(24.dp),
        verticalArrangement = Arrangement.SpaceBetween
    ) {
        Column {
            Spacer(modifier = Modifier.height(48.dp))
            Text(
                text = "Welcome to",
                style = MaterialTheme.typography.headlineMedium,
                color = TextSecondary
            )
            Text(
                text = "Android PC Environment",
                style = MaterialTheme.typography.headlineLarge,
                color = TextPrimary,
                fontWeight = FontWeight.Bold
            )
            Spacer(modifier = Modifier.height(16.dp))
            Text(
                text = "A serious, rootless Linux desktop runtime powered by a high-performance Rust core engine.",
                style = MaterialTheme.typography.bodyLarge,
                color = TextSecondary
            )

            Spacer(modifier = Modifier.height(32.dp))

            FeatureBullet(
                title = "Rootless Architecture",
                description = "Runs distributions inside isolated user namespaces without rooting or flashing."
            )
            Spacer(modifier = Modifier.height(16.dp))
            FeatureBullet(
                title = "Rust-First Engine",
                description = "Process supervision, memory safety, and native low-latency I/O orchestration."
            )
            Spacer(modifier = Modifier.height(16.dp))
            FeatureBullet(
                title = "Full Desktop Experience",
                description = "Integrated Xvnc display with XFCE desktop environment support."
            )
        }

        Button(
            onClick = onGetStarted,
            modifier = Modifier
                .fillMaxWidth()
                .height(52.dp),
            colors = ButtonDefaults.buttonColors(containerColor = AccentCyan),
            shape = RoundedCornerShape(12.dp)
        ) {
            Text(
                text = "Enter Dashboard",
                color = BgDark,
                fontWeight = FontWeight.Bold,
                style = MaterialTheme.typography.titleMedium
            )
        }
    }
}

@Composable
fun FeatureBullet(title: String, description: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(SurfaceDark, RoundedCornerShape(10.dp))
            .padding(14.dp),
        verticalAlignment = Alignment.Top
    ) {
        Box(
            modifier = Modifier
                .size(8.dp)
                .background(AccentCyan, RoundedCornerShape(4.dp))
                .padding(top = 6.dp)
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column {
            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium,
                color = TextPrimary,
                fontWeight = FontWeight.SemiBold
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = description,
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary
            )
        }
    }
}
