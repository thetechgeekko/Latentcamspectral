package com.example.latent.camera

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.SurfaceTexture
import android.opengl.EGL14
import android.util.Log
import android.view.Surface
import android.view.TextureView
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.example.latent.theme.*

/**
 * A Composable that renders a live Camera2 preview through an OpenGL LUT pipeline.
 */
@Composable
fun CameraViewfinder(
    modifier: Modifier = Modifier,
    controller: Camera2Controller,
    selectedLens: PhysicalLens?,
    currentLut: FloatArray? = null,
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current

    var hasPermission by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED
        )
    }

    // OpenGL state
    var renderer by remember { mutableStateOf<GLESPreviewRenderer?>(null) }
    var eglCore by remember { mutableStateOf<EglCore?>(null) }
    var cameraSurfaceTexture by remember { mutableStateOf<SurfaceTexture?>(null) }

    // Permission launcher
    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        hasPermission = granted
    }

    LaunchedEffect(Unit) {
        if (!hasPermission) {
            permissionLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    // Update LUT on renderer when it changes
    LaunchedEffect(currentLut, renderer) {
        if (currentLut != null && renderer != null) {
            renderer?.updateLut(currentLut, 33)
        }
    }

    // Lifecycle cleanup
    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_PAUSE) {
                controller.closeCamera()
                controller.stopBackgroundThread()
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
            controller.closeCamera()
            controller.stopBackgroundThread()
            renderer?.release()
            eglCore?.release()
        }
    }

    Box(
        modifier = modifier
            .clip(RoundedCornerShape(20.dp))
            .background(CameraDarkGray)
            .border(
                width = 1.dp,
                brush = Brush.linearGradient(
                    colors = listOf(CameraGray, CameraMidGray.copy(alpha = 0.3f), CameraGray)
                ),
                shape = RoundedCornerShape(20.dp),
            ),
        contentAlignment = Alignment.Center,
    ) {
        if (!EglCore.isGles3Supported(LocalContext.current)) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text(
                    "This device does not support OpenGL ES 3.0.\nLive preview is unavailable.",
                    color = Color.White,
                    textAlign = TextAlign.Center,
                )
            }
            return
        }

        if (hasPermission) {
            AndroidView(
                factory = { ctx ->
                    TextureView(ctx).also { tv ->
                        tv.surfaceTextureListener = object : TextureView.SurfaceTextureListener {
                            override fun onSurfaceTextureAvailable(surface: SurfaceTexture, width: Int, height: Int) {
                                // 1. Init EGL & Renderer
                                val core = EglCore()
                                eglCore = core
                                val windowSurface = core.createWindowSurface(surface)
                                core.makeCurrent(windowSurface)

                                val r = GLESPreviewRenderer()
                                r.init()
                                renderer = r

                                // 2. Create Camera Input Texture
                                val camST = SurfaceTexture(r.getTextureId())
                                cameraSurfaceTexture = camST
                                
                                // 3. Start Render Loop
                                camST.setOnFrameAvailableListener {
                                    core.makeCurrent(windowSurface)
                                    it.updateTexImage()
                                    r.draw(it)
                                    core.swapBuffers(windowSurface)
                                }

                                // 4. Open Camera
                                if (selectedLens != null) {
                                    controller.startBackgroundThread()
                                    val previewSize = controller.getPreviewSize(selectedLens.cameraId, 1920, 1080)
                                    camST.setDefaultBufferSize(previewSize.width, previewSize.height)
                                    controller.openCamera(selectedLens, Surface(camST))
                                }
                            }

                            override fun onSurfaceTextureSizeChanged(s: SurfaceTexture, w: Int, h: Int) {}
                            override fun onSurfaceTextureDestroyed(s: SurfaceTexture): Boolean {
                                controller.closeCamera()
                                renderer?.release()
                                eglCore?.release()
                                return true
                            }
                            override fun onSurfaceTextureUpdated(s: SurfaceTexture) {}
                        }
                    }
                },
                modifier = Modifier.fillMaxSize(),
            )
        } else {
            Text("CAMERA ACCESS REQUIRED", color = CameraMidGray)
        }

        // Focus marks
        Canvas(modifier = Modifier.fillMaxSize().padding(24.dp)) {
            val strokeWidth = 1.5f
            val markLen = 20.dp.toPx()
            val col = CameraLightGray.copy(alpha = 0.4f)
            drawLine(col, Offset(0f, 0f), Offset(markLen, 0f), strokeWidth, StrokeCap.Round)
            drawLine(col, Offset(0f, 0f), Offset(0f, markLen), strokeWidth, StrokeCap.Round)
            drawLine(col, Offset(size.width, 0f), Offset(size.width - markLen, 0f), strokeWidth, StrokeCap.Round)
            drawLine(col, Offset(size.width, 0f), Offset(size.width, markLen), strokeWidth, StrokeCap.Round)
            drawLine(col, Offset(0f, size.height), Offset(markLen, size.height), strokeWidth, StrokeCap.Round)
            drawLine(col, Offset(0f, size.height), Offset(0f, size.height - markLen), strokeWidth, StrokeCap.Round)
            drawLine(col, Offset(size.width, size.height), Offset(size.width - markLen, size.height), strokeWidth, StrokeCap.Round)
            drawLine(col, Offset(size.width, size.height), Offset(size.width, size.height - markLen), strokeWidth, StrokeCap.Round)
        }
    }
}
