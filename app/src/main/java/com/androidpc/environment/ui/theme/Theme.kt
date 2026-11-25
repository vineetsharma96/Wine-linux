package com.androidpc.environment.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp

private val DarkColorScheme = darkColorScheme(
    primary = AccentCyan,
    onPrimary = BgDark,
    primaryContainer = AccentPurple.copy(alpha = 0.22f),
    onPrimaryContainer = TextPrimary,
    secondary = AccentPurple,
    onSecondary = TextPrimary,
    secondaryContainer = SurfaceContainer,
    onSecondaryContainer = TextPrimary,
    tertiary = StarSilver,
    onTertiary = BgDark,
    surface = SurfaceDark,
    onSurface = TextPrimary,
    background = BgDark,
    onBackground = TextPrimary,
    surfaceVariant = SurfaceContainer,
    onSurfaceVariant = TextSecondary,
    outline = CardBorder,
    outlineVariant = CardBorderActive,
    error = StatusRed,
    onError = TextPrimary
)

private val AppShapes = Shapes(
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(14.dp),
    large = RoundedCornerShape(18.dp),
    extraLarge = RoundedCornerShape(24.dp)
)

@Composable
fun AndroidPCEnvironmentTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = DarkColorScheme,
        typography = Typography,
        shapes = AppShapes,
        content = content
    )
}
