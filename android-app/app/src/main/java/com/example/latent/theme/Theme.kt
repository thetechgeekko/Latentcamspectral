package com.example.latent.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

// Latent is ALWAYS dark. There is no light theme for a camera app.
private val LatentDarkColorScheme = darkColorScheme(
    primary = LeicaRed,
    onPrimary = PureWhite,
    secondary = KodakGold,
    onSecondary = CameraBlack,
    tertiary = CameraLightGray,
    background = CameraBlack,
    onBackground = PureWhite,
    surface = CameraBody,
    onSurface = SoftWhite,
    surfaceVariant = CameraDarkGray,
    onSurfaceVariant = CameraLightGray,
    error = RecordingRed,
)

@Composable
fun LatentTheme(
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = LatentDarkColorScheme,
        typography = Typography,
        content = content,
    )
}
