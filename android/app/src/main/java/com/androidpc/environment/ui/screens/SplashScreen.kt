package com.androidpc.environment.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.androidpc.environment.ui.theme.AccentCyan
import com.androidpc.environment.ui.theme.BgDark
import com.androidpc.environment.ui.theme.TextMuted
import com.androidpc.environment.ui.theme.TextPrimary
import kotlinx.coroutines.delay

@Composable
fun SplashScreen(onSplashFinished: () -> Unit) {
    LaunchedEffect(Unit) {
        delay(1200)
        onSplashFinished()
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(BgDark),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Text(
                text = "ANDROID PC",
                style = MaterialTheme.typography.headlineLarge,
                color = AccentCyan,
                fontWeight = FontWeight.ExtraBold,
                letterSpacing = 4.sp
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = "ENVIRONMENT",
                style = MaterialTheme.typography.titleMedium,
                color = TextPrimary,
                letterSpacing = 6.sp
            )
            Spacer(modifier = Modifier.height(24.dp))
            CircularProgressIndicator(
                color = AccentCyan,
                strokeWidth = 2.dp,
                modifier = Modifier.size(28.dp)
            )
            Spacer(modifier = Modifier.height(16.dp))
            Text(
                text = "Starting Rust Core Engine...",
                style = MaterialTheme.typography.labelSmall,
                color = TextMuted
            )
        }
    }
}
