package com.example.latent.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

// Technical Sans-Serif for metadata (ISO, Shutter, EV)
// We use the system default which maps to Roboto on Android
private val TechnicalFamily = FontFamily.Default

// Serif for film stock names — evokes nostalgia
private val FilmNameFamily = FontFamily.Serif

val Typography = Typography(
    // Film stock name (e.g., "Portra 400") — large, elegant
    displayLarge = TextStyle(
        fontFamily = FilmNameFamily,
        fontWeight = FontWeight.Light,
        fontSize = 28.sp,
        letterSpacing = 1.5.sp,
    ),
    // Film stock name — medium variant
    displayMedium = TextStyle(
        fontFamily = FilmNameFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 22.sp,
        letterSpacing = 1.2.sp,
    ),
    // Section headers (e.g., "EXPOSURE", "LENS")
    titleLarge = TextStyle(
        fontFamily = TechnicalFamily,
        fontWeight = FontWeight.Bold,
        fontSize = 14.sp,
        letterSpacing = 2.sp,
    ),
    // Camera metadata labels (ISO, SS)
    titleMedium = TextStyle(
        fontFamily = TechnicalFamily,
        fontWeight = FontWeight.Medium,
        fontSize = 12.sp,
        letterSpacing = 1.5.sp,
    ),
    // Camera metadata values (e.g., "400", "1/125")
    bodyLarge = TextStyle(
        fontFamily = FontFamily.Monospace,
        fontWeight = FontWeight.Normal,
        fontSize = 16.sp,
        letterSpacing = 0.5.sp,
    ),
    // General body
    bodyMedium = TextStyle(
        fontFamily = TechnicalFamily,
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        lineHeight = 20.sp,
        letterSpacing = 0.25.sp,
    ),
    // Small labels (e.g., dial tick marks)
    labelSmall = TextStyle(
        fontFamily = FontFamily.Monospace,
        fontWeight = FontWeight.Medium,
        fontSize = 10.sp,
        letterSpacing = 0.5.sp,
    ),
)
