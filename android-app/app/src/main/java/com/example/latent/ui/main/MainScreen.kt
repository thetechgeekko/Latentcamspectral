package com.example.latent.ui.main

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.platform.LocalContext
import com.example.latent.camera.Camera2Controller
import com.example.latent.camera.CaptureCallback
import com.example.latent.camera.CameraViewfinder
import com.example.latent.camera.PhysicalLens
import com.example.latent.theme.*
import com.example.latent.processing.FILM_STOCKS
import com.example.latent.processing.DevelopmentQueue
import com.example.latent.processing.DevelopmentStatus
import com.example.latent.processing.RawJob
import kotlinx.coroutines.launch
import kotlin.math.roundToInt
import coil.compose.AsyncImage
import coil.request.ImageRequest
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.combinedClickable
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.example.latent.utils.LogManager

// ── Film stocks the user can cycle through ──────────────────────────────
private val filmStockNames = FILM_STOCKS.map { it.displayName }

@OptIn(ExperimentalMaterial3Api::class, androidx.compose.foundation.ExperimentalFoundationApi::class)
@Composable
fun CameraScreen(modifier: Modifier = Modifier, onGalleryOpen: () -> Unit = {}) {
    val context = LocalContext.current
    var selectedFilmIndex by remember { mutableIntStateOf(0) }
    var exposureComp by remember { mutableFloatStateOf(0f) }

    // Camera2 controller and lens discovery
    val controller = remember { Camera2Controller(context) }
    val discoveredLenses = remember { controller.discoverLenses() }
    var selectedLensIndex by remember { mutableIntStateOf(
        // Default to the main lens (usually the one around 24-28mm equiv)
        discoveredLenses.indexOfFirst { it.focalLength in 4f..8f }.coerceAtLeast(0)
    ) }
    val selectedLens = discoveredLenses.getOrNull(selectedLensIndex)

    // True only while the shutter is open / RAW is being captured (< 1 s).
    // Development state comes from DevelopmentQueue.status so the shutter stays free.
    var isCapturing by remember { mutableStateOf(false) }
    var frameCount by remember { mutableIntStateOf(24) }
    var lastSaveMessage by remember { mutableStateOf("") }
    var lastDevelopedUri by remember { mutableStateOf<String?>(null) }

    val developmentStatus by DevelopmentQueue.status.collectAsStateWithLifecycle()
    
    // Debug state
    var showDebugMenu by remember { mutableStateOf(false) }
    var isLoggingEnabled by remember { mutableStateOf(LogManager.isLoggingActive()) }

    // Spectral film processor
    val filmProcessor = remember { com.example.latent.processing.FilmProcessor(context) }
    val coroutineScope = rememberCoroutineScope()
    var previewLut by remember { mutableStateOf<FloatArray?>(null) }

    // Live preview update: Generate LUT when film or exposure changes
    LaunchedEffect(selectedFilmIndex, exposureComp) {
        filmProcessor.updateExposure(exposureComp)
        previewLut = filmProcessor.generatePreviewLut(selectedFilmIndex)
    }

    // Wire capture callback
    LaunchedEffect(controller) {
        controller.captureCallback = object : CaptureCallback {
            override fun onCaptureStarted() {
                isCapturing = true
                lastSaveMessage = "Capturing RAW…"
            }

            override fun onCaptureCompleted(savedUri: String) {
                // Archival DNG saved — nothing extra needed here
            }

            override fun onCaptureFailed(error: String) {
                isCapturing = false
                lastSaveMessage = error
            }

            override fun onRawExtracted(
                pixels: ShortArray,
                width: Int,
                height: Int,
                result: android.hardware.camera2.TotalCaptureResult,
                characteristics: android.hardware.camera2.CameraCharacteristics,
            ) {
                isCapturing = false
                frameCount = (frameCount - 1).coerceAtLeast(0)
                lastSaveMessage = "Queued for darkroom"
                val job = RawJob(pixels, width, height, result, characteristics, selectedFilmIndex)
                DevelopmentQueue.enqueue(job, context)
            }
        }
    }

    if (showDebugMenu) {
        androidx.compose.ui.window.Dialog(
            onDismissRequest = { showDebugMenu = false }
        ) {
            Surface(
                modifier = Modifier
                    .padding(24.dp)
                    .fillMaxWidth(),
                shape = RoundedCornerShape(16.dp),
                color = CameraDarkGray,
                contentColor = PureWhite
            ) {
                Column(modifier = Modifier.padding(24.dp)) {
                    Text("DEVELOPER TOOLS", style = MaterialTheme.typography.titleLarge, color = LeicaRed)
                    Spacer(Modifier.height(16.dp))
                    
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text("System Logging", style = MaterialTheme.typography.bodyLarge)
                            Text("Capture Rust & Camera2 logs", style = MaterialTheme.typography.labelSmall, color = CameraMidGray)
                        }
                        Switch(
                            checked = isLoggingEnabled,
                            onCheckedChange = {
                                isLoggingEnabled = it
                                LogManager.setLoggingActive(it)
                            },
                            colors = SwitchDefaults.colors(checkedThumbColor = LeicaRed)
                        )
                    }
                    
                    Spacer(Modifier.height(24.dp))
                    
                    Button(
                        onClick = {
                            coroutineScope.launch {
                                val path = LogManager.exportLogs(context)
                                if (path != null) {
                                    lastSaveMessage = "Logs Exported"
                                    showDebugMenu = false
                                }
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                        colors = ButtonDefaults.buttonColors(containerColor = CameraGray),
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Text("EXPORT LOGS TO DOWNLOADS")
                    }
                    
                    TextButton(
                        onClick = { showDebugMenu = false },
                        modifier = Modifier.align(Alignment.End)
                    ) {
                        Text("CLOSE", color = CameraMidGray)
                    }
                }
            }
        }
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(CameraBlack),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        // ── Top status bar ──────────────────────────────────────
        TopStatusBar(
            filmStock = filmStockNames[selectedFilmIndex],
            frameCount = frameCount,
            developmentStatus = developmentStatus,
            onTitleLongClick = { showDebugMenu = true }
        )

        Spacer(modifier = Modifier.height(8.dp))

        // ── Live Camera2 Viewfinder (rounded corners + OpenGL LUT) ───────────
        CameraViewfinder(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 16.dp)
                .aspectRatio(4f / 3f),
            controller = controller,
            selectedLens = selectedLens,
            currentLut = previewLut,
        )

        Spacer(modifier = Modifier.height(8.dp))

        // ── Darkroom progress bar ───────────────────────────────
        if (developmentStatus.isProcessing || developmentStatus.queueSize > 0) {
            LinearProgressIndicator(
                progress = { developmentStatus.progress },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp)
                    .height(2.dp),
                color = LeicaRed,
                trackColor = CameraDarkGray,
            )
            Spacer(modifier = Modifier.height(8.dp))
        } else {
            Spacer(modifier = Modifier.height(10.dp))
        }

        // ── Exposure Compensation Dial ──────────────────────────
        ExposureDial(
            value = exposureComp,
            onValueChange = { exposureComp = it },
        )

        Spacer(modifier = Modifier.weight(1f))

        // ── Film Selector Carousel ──────────────────────────────
        FilmSelector(
            stocks = filmStockNames,
            selectedIndex = selectedFilmIndex,
            onSelected = { selectedFilmIndex = it },
        )

        Spacer(modifier = Modifier.height(24.dp))

        // ── Shutter + Controls Row ──────────────────────────────
        BottomControls(
            lenses = discoveredLenses,
            selectedLensIndex = selectedLensIndex,
            onLensSelected = { selectedLensIndex = it },
            onShutterPressed = {
                if (!isCapturing) {
                    controller.captureRawPhoto()
                }
            },
            isCapturing = isCapturing,
            queueSize = developmentStatus.queueSize,
            onGalleryOpen = onGalleryOpen,
        )

        Spacer(modifier = Modifier.height(32.dp))
    }
}

// ════════════════════════════════════════════════════════════════════════
// Composable Components
// ════════════════════════════════════════════════════════════════════════

@Composable
private fun TopStatusBar(
    filmStock: String,
    frameCount: Int,
    developmentStatus: DevelopmentStatus,
    onTitleLongClick: () -> Unit = {},
) {
    val isDeveloping = developmentStatus.isProcessing || developmentStatus.queueSize > 0
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "LATENT",
            style = MaterialTheme.typography.titleLarge,
            color = if (isDeveloping) CameraMidGray else PureWhite,
            modifier = Modifier.combinedClickable(
                onClick = {},
                onLongClick = onTitleLongClick
            )
        )
        if (isDeveloping) {
            val pct = (developmentStatus.progress * 100).toInt()
            val n = developmentStatus.queueSize
            Text(
                text = if (n > 1) "DARKROOM ($n) · $pct%" else "DARKROOM · $pct%",
                style = MaterialTheme.typography.titleMedium,
                color = LeicaRed,
                letterSpacing = 2.sp,
            )
        } else {
            Text(
                text = filmStock.uppercase(),
                style = MaterialTheme.typography.titleMedium,
                color = SoftWhite,
                letterSpacing = 2.sp,
            )
        }
        Text(
            text = if (isDeveloping) "◉" else "●",
            style = MaterialTheme.typography.bodyLarge,
            color = if (isDeveloping) RecordingRed else LeicaRed,
        )
    }
}



@Composable
private fun ExposureDial(value: Float, onValueChange: (Float) -> Unit) {
    val haptic = LocalHapticFeedback.current
    var lastNotch by remember { mutableIntStateOf(0) }

    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = "EXPOSURE",
            style = MaterialTheme.typography.titleMedium,
            color = CameraMidGray,
        )
        Spacer(modifier = Modifier.height(6.dp))

        // The horizontal dial strip
        Box(
            modifier = Modifier
                .width(280.dp)
                .height(40.dp)
                .clip(RoundedCornerShape(8.dp))
                .background(CameraBody)
                .pointerInput(Unit) {
                    detectDragGestures { change, dragAmount ->
                        change.consume()
                        val newVal = (value + dragAmount.x / 200f).coerceIn(-3f, 3f)
                        val newNotch = (newVal * 3).roundToInt()
                        if (newNotch != lastNotch) {
                            haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                            lastNotch = newNotch
                        }
                        onValueChange(newVal)
                    }
                },
            contentAlignment = Alignment.Center,
        ) {
            // Tick marks
            Canvas(modifier = Modifier.fillMaxSize()) {
                val steps = 13 // -3 to +3 in 1/2 stops
                val spacing = size.width / (steps - 1)
                for (i in 0 until steps) {
                    val x = i * spacing
                    val isFull = (i - 6) % 2 == 0
                    val height = if (isFull) size.height * 0.5f else size.height * 0.3f
                    val col = if (i == 6) LeicaRed else CameraMidGray
                    drawLine(
                        col,
                        Offset(x, (size.height - height) / 2f),
                        Offset(x, (size.height + height) / 2f),
                        strokeWidth = if (isFull) 2f else 1f,
                        cap = StrokeCap.Round,
                    )
                }
            }

            // Current value indicator
            val displayValue = if (value >= 0) "+%.1f".format(value) else "%.1f".format(value)
            Text(
                text = "$displayValue EV",
                style = MaterialTheme.typography.labelSmall,
                color = PureWhite,
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(bottom = 2.dp),
            )
        }
    }
}

@Composable
private fun FilmSelector(
    stocks: List<String>,
    selectedIndex: Int,
    onSelected: (Int) -> Unit,
) {
    val haptic = LocalHapticFeedback.current

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 24.dp),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // Previous arrow
        Text(
            text = "◂",
            fontSize = 20.sp,
            color = if (selectedIndex > 0) SoftWhite else CameraMidGray,
            modifier = Modifier
                .clickable(enabled = selectedIndex > 0) {
                    haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                    onSelected(selectedIndex - 1)
                }
                .padding(12.dp),
        )

        // Film name
        Box(
            modifier = Modifier.weight(1f),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = stocks[selectedIndex],
                style = MaterialTheme.typography.displayMedium,
                color = KodakGold,
                textAlign = TextAlign.Center,
            )
        }

        // Next arrow
        Text(
            text = "▸",
            fontSize = 20.sp,
            color = if (selectedIndex < stocks.size - 1) SoftWhite else CameraMidGray,
            modifier = Modifier
                .clickable(enabled = selectedIndex < stocks.size - 1) {
                    haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                    onSelected(selectedIndex + 1)
                }
                .padding(12.dp),
        )
    }
}

@Composable
private fun BottomControls(
    lenses: List<PhysicalLens>,
    selectedLensIndex: Int,
    onLensSelected: (Int) -> Unit,
    onShutterPressed: () -> Unit,
    isCapturing: Boolean,
    queueSize: Int,
    onGalleryOpen: () -> Unit = {},
) {
    val haptic = LocalHapticFeedback.current

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 40.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // Gallery thumbnail — tapping opens the Gallery screen
        Box(
            modifier = Modifier
                .size(48.dp)
                .clip(RoundedCornerShape(10.dp))
                .background(CameraGray)
                .clickable { onGalleryOpen() },
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "⊞",
                color = CameraLightGray,
                fontSize = 20.sp,
            )
        }

        // ── The Shutter Button with queue badge ──
        Box(contentAlignment = Alignment.TopEnd) {
            ShutterButton(
                onClick = onShutterPressed,
                isCapturing = isCapturing,
            )
            if (queueSize > 0) {
                Box(
                    modifier = Modifier
                        .offset(x = 4.dp, y = (-4).dp)
                        .size(20.dp)
                        .clip(CircleShape)
                        .background(LeicaRed),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = queueSize.toString(),
                        color = PureWhite,
                        fontSize = 10.sp,
                    )
                }
            }
        }

        // Lens selector — cycle through discovered physical lenses
        val currentLens = lenses.getOrNull(selectedLensIndex)
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.clickable {
                if (lenses.isNotEmpty()) {
                    haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                    onLensSelected((selectedLensIndex + 1) % lenses.size)
                }
            }.padding(8.dp),
        ) {
            Text(
                text = currentLens?.label ?: "--",
                style = MaterialTheme.typography.labelSmall,
                color = PureWhite,
            )
            Text(
                text = if (currentLens?.supportsRaw == true) "RAW" else "JPEG",
                style = MaterialTheme.typography.labelSmall,
                color = if (currentLens?.supportsRaw == true) KodakGold else CameraMidGray,
            )
        }
    }
}

@Composable
private fun ShutterButton(onClick: () -> Unit, isCapturing: Boolean = false) {
    val interactionSource = remember { MutableInteractionSource() }
    val isPressed by interactionSource.collectIsPressedAsState()
    val haptic = LocalHapticFeedback.current

    val scale by animateFloatAsState(
        targetValue = if (isPressed) 0.92f else 1f,
        animationSpec = tween(durationMillis = 80),
        label = "shutterScale",
    )
    val ringColor by animateColorAsState(
        targetValue = when {
            isCapturing -> RecordingRed
            isPressed -> LeicaRed
            else -> ShutterRing
        },
        animationSpec = tween(durationMillis = 80),
        label = "shutterRing",
    )

    Box(
        modifier = Modifier
            .size((72 * scale).dp)
            .clip(CircleShape)
            .border(3.dp, ringColor, CircleShape)
            .clickable(
                interactionSource = interactionSource,
                indication = null,
                enabled = !isCapturing,
            ) {
                haptic.performHapticFeedback(HapticFeedbackType.LongPress)
                onClick()
            },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier = Modifier
                .size((58 * scale).dp)
                .clip(CircleShape)
                .background(
                    Brush.radialGradient(
                        colors = if (isCapturing) listOf(
                            RecordingRed.copy(alpha = 0.6f),
                            RecordingRed.copy(alpha = 0.3f),
                        ) else listOf(
                            ShutterRing,
                            ShutterRing.copy(alpha = 0.85f),
                        ),
                    )
                ),
        )
    }
}
