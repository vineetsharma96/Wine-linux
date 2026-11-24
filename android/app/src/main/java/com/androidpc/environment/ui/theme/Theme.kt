package com.androidpc.environment.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

private val DarkColorScheme = darkColorScheme(
    primary = AccentCyan,
    onPrimary = BgDark,
    primaryContainer = AccentBlue,
    onPrimaryContainer = TextPrimary,
    surface = SurfaceDark,
    onSurface = TextPrimary,
    background = BgDark,
    onBackground = TextPrimary,
    surfaceVariant = SurfaceContainer,
    onSurfaceVariant = TextSecondary,
    outline = CardBorder
)

@Composable
fun AndroidPCEnvironmentTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = DarkColorScheme,
        typography = Typography,
        content = content
    )
}
