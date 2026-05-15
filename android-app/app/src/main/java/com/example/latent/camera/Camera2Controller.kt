package com.example.latent.camera

import android.annotation.SuppressLint
import android.content.ContentValues
import android.content.Context
import android.graphics.ImageFormat
import android.graphics.SurfaceTexture
import android.hardware.camera2.*
import android.hardware.camera2.params.OutputConfiguration
import android.hardware.camera2.params.SessionConfiguration
import android.media.Image
import android.media.ImageReader
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.provider.MediaStore
import android.util.Log
import android.util.Size
import android.view.Surface
import java.io.IOException
import java.text.SimpleDateFormat
import java.util.*
import java.util.concurrent.Executor
import java.util.concurrent.Executors
import java.util.concurrent.Semaphore
import java.util.concurrent.TimeUnit

/**
 * Represents a physical lens on the device.
 */
data class PhysicalLens(
    val cameraId: String,
    val physicalId: String?,
    val focalLength: Float,
    val aperture: Float,
    val facing: Int,
    val label: String,
    val isLogical: Boolean,
    val supportsRaw: Boolean,
    val rawSize: Size?,
)

/** Capture state callback for the UI layer. */
interface CaptureCallback {
    fun onCaptureStarted()
    fun onCaptureCompleted(savedUri: String)
    fun onCaptureFailed(error: String)
    fun onRawCaptured(image: android.media.Image, result: android.hardware.camera2.TotalCaptureResult, characteristics: android.hardware.camera2.CameraCharacteristics) {}
}

/**
 * Low-level Camera2 controller.
 *
 * Discovers physical lenses (bypassing logical camera auto-switching),
 * opens a specific sensor, streams preview, and captures RAW photos
 * locked to a specific physical camera ID.
 */
class Camera2Controller(private val context: Context) {

    companion object {
        private const val TAG = "Camera2Controller"
    }

    private val cameraManager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
    private var cameraDevice: CameraDevice? = null
    private var captureSession: CameraCaptureSession? = null
    private var backgroundThread: HandlerThread? = null
    private var backgroundHandler: Handler? = null
    private val openLock = Semaphore(1)
    private val executor: Executor = Executors.newSingleThreadExecutor()

    // RAW capture
    private var rawImageReader: ImageReader? = null
    // JPEG capture for spectral engine processing
    private var jpegImageReader: ImageReader? = null
    private var previewSurfaceRef: Surface? = null
    var captureCallback: CaptureCallback? = null

    var currentLens: PhysicalLens? = null
        private set

    // ── Lens Discovery ──────────────────────────────────────────────────

    /**
     * Enumerates all physical rear-facing lenses on the device.
     * On Samsung Ultra devices, this looks inside the logical camera's
     * physical camera IDs to find the hidden sensors.
     */
    fun discoverLenses(): List<PhysicalLens> {
        val lenses = mutableListOf<PhysicalLens>()

        for (cameraId in cameraManager.cameraIdList) {
            val chars = cameraManager.getCameraCharacteristics(cameraId)
            val facing = chars.get(CameraCharacteristics.LENS_FACING) ?: continue

            // Only rear-facing cameras
            if (facing != CameraCharacteristics.LENS_FACING_BACK) continue

            val focalLengths = chars.get(CameraCharacteristics.LENS_INFO_AVAILABLE_FOCAL_LENGTHS) ?: floatArrayOf()
            val apertures = chars.get(CameraCharacteristics.LENS_INFO_AVAILABLE_APERTURES) ?: floatArrayOf()
            val physicalIds = chars.physicalCameraIds

            if (physicalIds.isNotEmpty()) {
                // Logical camera with hidden physical lenses
                for (physId in physicalIds) {
                    try {
                        val physChars = cameraManager.getCameraCharacteristics(physId)
                        val physFocal = physChars.get(CameraCharacteristics.LENS_INFO_AVAILABLE_FOCAL_LENGTHS)?.firstOrNull() ?: 0f
                        val physAperture = physChars.get(CameraCharacteristics.LENS_INFO_AVAILABLE_APERTURES)?.firstOrNull() ?: 0f
                        val label = focalLengthToLabel(physFocal)

                        // Check RAW support for this physical sensor
                        val capabilities = physChars.get(CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES)
                        val supportsRaw = capabilities?.contains(
                            CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES_RAW
                        ) == true
                        val rawSize = if (supportsRaw) {
                            val map = physChars.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
                            map?.getOutputSizes(ImageFormat.RAW_SENSOR)?.maxByOrNull { it.width * it.height }
                        } else null

                        lenses.add(PhysicalLens(
                            cameraId = cameraId,
                            physicalId = physId,
                            focalLength = physFocal,
                            aperture = physAperture,
                            facing = facing,
                            label = label,
                            isLogical = false,
                            supportsRaw = supportsRaw,
                            rawSize = rawSize,
                        ))
                    } catch (e: Exception) {
                        Log.w(TAG, "Failed to query physical camera $physId: ${e.message}")
                    }
                }
            } else {
                // Standalone camera (no physical sub-cameras)
                val capabilities = chars.get(CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES)
                val supportsRaw = capabilities?.contains(
                    CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES_RAW
                ) == true
                val rawSize = if (supportsRaw) {
                    val map = chars.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
                    map?.getOutputSizes(ImageFormat.RAW_SENSOR)?.maxByOrNull { it.width * it.height }
                } else null

                lenses.add(PhysicalLens(
                    cameraId = cameraId,
                    physicalId = null,
                    focalLength = focalLengths.firstOrNull() ?: 0f,
                    aperture = apertures.firstOrNull() ?: 0f,
                    facing = facing,
                    label = focalLengthToLabel(focalLengths.firstOrNull() ?: 0f),
                    isLogical = true,
                    supportsRaw = supportsRaw,
                    rawSize = rawSize,
                ))
            }
        }

        // Sort by focal length (ultrawide first, then main, then tele)
        return lenses.sortedBy { it.focalLength }
    }

    /**
     * Gets the best preview size for the given camera that fits within maxWidth x maxHeight.
     */
    fun getPreviewSize(cameraId: String, maxWidth: Int, maxHeight: Int): Size {
        val chars = cameraManager.getCameraCharacteristics(cameraId)
        val map = chars.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP) ?: return Size(1920, 1080)
        val sizes = map.getOutputSizes(SurfaceTexture::class.java) ?: return Size(1920, 1080)

        val targetRatio = 4.0 / 3.0
        return sizes
            .filter { it.width <= maxWidth && it.height <= maxHeight }
            .minByOrNull { Math.abs(it.width.toDouble() / it.height.toDouble() - targetRatio) }
            ?: sizes.first()
    }

    // ── Camera Lifecycle ────────────────────────────────────────────────

    fun startBackgroundThread() {
        backgroundThread = HandlerThread("CameraBackground").also { it.start() }
        backgroundHandler = Handler(backgroundThread!!.looper)
    }

    fun stopBackgroundThread() {
        backgroundThread?.quitSafely()
        try {
            backgroundThread?.join()
            backgroundThread = null
            backgroundHandler = null
        } catch (e: InterruptedException) {
            Log.e(TAG, "Background thread interrupted", e)
        }
    }

    @SuppressLint("MissingPermission")
    fun openCamera(lens: PhysicalLens, previewSurface: Surface) {
        if (!openLock.tryAcquire(2500, TimeUnit.MILLISECONDS)) {
            throw RuntimeException("Timed out waiting to lock camera opening.")
        }

        currentLens = lens
        previewSurfaceRef = previewSurface

        // Set up RAW ImageReader if this lens supports RAW
        rawImageReader?.close()
        rawImageReader = null
        if (lens.supportsRaw && lens.rawSize != null) {
            rawImageReader = ImageReader.newInstance(
                lens.rawSize.width, lens.rawSize.height,
                ImageFormat.RAW_SENSOR, 2
            )
            rawImageReader!!.setOnImageAvailableListener({ reader ->
                val image = reader.acquireLatestImage() ?: return@setOnImageAvailableListener
                val res = captureResult as? TotalCaptureResult
                val lens = currentLens
                if (res != null && lens != null) {
                    val chars = cameraManager.getCameraCharacteristics(lens.physicalId ?: lens.cameraId)
                    // 1. Archival Save (DNG)
                    saveRawImage(image, res, chars)
                    // 2. Authentic Processing (Raw Image)
                    captureCallback?.onRawCaptured(image, res, chars)
                }
                image.close()
            }, backgroundHandler)
        }

        // Kill the JPEG reader — we are authentic now
        jpegImageReader?.close()
        jpegImageReader = null

        cameraManager.openCamera(lens.cameraId, object : CameraDevice.StateCallback() {
            override fun onOpened(camera: CameraDevice) {
                openLock.release()
                cameraDevice = camera
                createCaptureSession(camera, previewSurface, lens)
            }

            override fun onDisconnected(camera: CameraDevice) {
                openLock.release()
                camera.close()
                cameraDevice = null
            }

            override fun onError(camera: CameraDevice, error: Int) {
                openLock.release()
                camera.close()
                cameraDevice = null
                Log.e(TAG, "Camera device error: $error")
            }
        }, backgroundHandler)
    }

    fun closeCamera() {
        try {
            openLock.acquire()
            captureSession?.close()
            captureSession = null
            cameraDevice?.close()
            cameraDevice = null
            rawImageReader?.close()
            rawImageReader = null
            jpegImageReader?.close()
            jpegImageReader = null
        } catch (e: InterruptedException) {
            throw RuntimeException("Interrupted while closing camera.", e)
        } finally {
            openLock.release()
        }
    }

    // ── Session with Physical Camera ID Locking ─────────────────────────

    private fun createCaptureSession(camera: CameraDevice, previewSurface: Surface, lens: PhysicalLens) {
        val outputs = mutableListOf<OutputConfiguration>()

        // Preview output
        val previewOutput = OutputConfiguration(previewSurface)
        outputs.add(previewOutput)

        // RAW output — LOCKED to the specific physical sensor
        val rawReader = rawImageReader
        if (rawReader != null && lens.physicalId != null) {
            val rawOutput = OutputConfiguration(rawReader.surface)
            // ═══════════════════════════════════════════════════════════
            // THE MAGIC: Lock RAW stream to this exact physical sensor.
            // Samsung CANNOT auto-switch when this is set.
            // ═══════════════════════════════════════════════════════════
            rawOutput.setPhysicalCameraId(lens.physicalId)
            outputs.add(rawOutput)
            Log.i(TAG, "RAW stream locked to physical camera: ${lens.physicalId} (${lens.label})")
        } else if (rawReader != null) {
            // Standalone camera — no physical ID needed
            outputs.add(OutputConfiguration(rawReader.surface))
        }

        // JPEG output for spectral engine processing
        val jpegReader = jpegImageReader
        if (jpegReader != null) {
            outputs.add(OutputConfiguration(jpegReader.surface))
        }

        val sessionConfig = SessionConfiguration(
            SessionConfiguration.SESSION_REGULAR,
            outputs,
            executor,
            object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(session: CameraCaptureSession) {
                    if (cameraDevice == null) return
                    captureSession = session
                    startPreview(session, previewSurface)
                }

                override fun onConfigureFailed(session: CameraCaptureSession) {
                    Log.e(TAG, "Capture session configuration failed")
                }
            },
        )

        camera.createCaptureSession(sessionConfig)
    }

    private fun startPreview(session: CameraCaptureSession, previewSurface: Surface) {
        try {
            val previewRequest = session.device.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW)
            previewRequest.addTarget(previewSurface)
            previewRequest.set(
                CaptureRequest.CONTROL_AF_MODE,
                CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE,
            )
            session.setRepeatingRequest(previewRequest.build(), null, backgroundHandler)
        } catch (e: CameraAccessException) {
            Log.e(TAG, "Failed to start preview", e)
        }
    }

    // ── RAW Capture (Shutter Press) ─────────────────────────────────────

    /**
     * Captures a single RAW frame from the currently locked physical sensor.
     * The preview keeps running — no interruption.
     */
    fun captureRawPhoto() {
        val session = captureSession ?: run {
            captureCallback?.onCaptureFailed("No active capture session")
            return
        }

        captureCallback?.onCaptureStarted()

        try {
            val captureRequest = session.device.createCaptureRequest(CameraDevice.TEMPLATE_STILL_CAPTURE)

            // Add RAW target
            rawImageReader?.let { captureRequest.addTarget(it.surface) }

            // Also keep the preview alive during capture
            previewSurfaceRef?.let { captureRequest.addTarget(it) }

            // Lock focus for the capture
            captureRequest.set(CaptureRequest.CONTROL_AF_MODE, CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE)

            session.capture(captureRequest.build(), object : CameraCaptureSession.CaptureCallback() {
                override fun onCaptureCompleted(
                    session: CameraCaptureSession,
                    request: CaptureRequest,
                    result: TotalCaptureResult,
                ) {
                    captureResult = result
                    Log.i(TAG, "Capture completed")
                }

                override fun onCaptureFailed(
                    session: CameraCaptureSession,
                    request: CaptureRequest,
                    failure: CaptureFailure,
                ) {
                    captureCallback?.onCaptureFailed("Capture failed: reason ${failure.reason}")
                }
            }, backgroundHandler)
        } catch (e: CameraAccessException) {
            captureCallback?.onCaptureFailed("CameraAccessException: ${e.message}")
        }
    }

    // ── Save RAW Image ──────────────────────────────────────────────────

    private fun saveRawImage(image: Image, result: TotalCaptureResult, chars: CameraCharacteristics) {
        val timestamp = SimpleDateFormat("yyyyMMdd_HHmmss_SSS", Locale.US).format(Date())
        val filename = "LATENT_RAW_${timestamp}.dng"

        try {
            // Save DNG to MediaStore
            val contentValues = ContentValues().apply {
                put(MediaStore.Images.Media.DISPLAY_NAME, filename)
                put(MediaStore.Images.Media.MIME_TYPE, "image/x-adobe-dng")
                put(MediaStore.Images.Media.RELATIVE_PATH, "Pictures/Latent/RAW")
                put(MediaStore.Images.Media.IS_PENDING, 1)
            }

            val resolver = context.contentResolver
            val uri = resolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, contentValues) ?: return

            resolver.openOutputStream(uri)?.use { outputStream ->
                val dngCreator = android.hardware.camera2.DngCreator(chars, result)
                dngCreator.setDescription("Captured by Latent Cam (Authentic RAW)")
                dngCreator.writeImage(outputStream, image)
                dngCreator.close()
            }

            contentValues.clear()
            contentValues.put(MediaStore.Images.Media.IS_PENDING, 0)
            resolver.update(uri, contentValues, null, null)

            captureCallback?.onCaptureCompleted(uri.toString())
            Log.i(TAG, "RAW DNG saved: $filename")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save RAW image", e)
        }
    }

    // Store the latest capture result for DNG metadata
    private var captureResult: CaptureResult? = null

    // ── Helpers ──────────────────────────────────────────────────────────

    private fun focalLengthToLabel(focalLengthMm: Float): String {
        val equiv = (focalLengthMm * 6.5f).toInt()
        return when {
            equiv < 16 -> "${equiv}mm UW"
            equiv < 30 -> "${equiv}mm"
            equiv < 60 -> "${equiv}mm"
            equiv < 100 -> "${equiv}mm Tele"
            else -> "${equiv}mm Periscope"
        }
    }
}
