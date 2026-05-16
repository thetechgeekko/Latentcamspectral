package com.example.latent.processing

import android.content.ContentValues
import android.content.Context
import android.graphics.Bitmap
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CaptureResult
import android.hardware.camera2.TotalCaptureResult
import android.media.Image
import android.provider.MediaStore
import android.util.Log
import androidx.exifinterface.media.ExifInterface
import com.latentcam.spectralfilm.SpectralEngine
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.nio.ShortBuffer
import java.text.SimpleDateFormat
import java.util.*

data class FilmStock(
    val displayName: String,
    val filmProfile: String,
    val printProfile: String,
)

val FILM_STOCKS = listOf(
    // ── Kodak Color Negative ───────────────────────────────────────────────
    FilmStock("Kodak Portra 160",        "kodak_portra_160.json",        "kodak_portra_endura.json"),
    FilmStock("Kodak Portra 400",        "kodak_portra_400.json",        "kodak_portra_endura.json"),
    FilmStock("Kodak Portra 800",        "kodak_portra_800.json",        "kodak_portra_endura.json"),
    FilmStock("Kodak Portra 800 +1",     "kodak_portra_800_push1.json",  "kodak_portra_endura.json"),
    FilmStock("Kodak Portra 800 +2",     "kodak_portra_800_push2.json",  "kodak_portra_endura.json"),
    FilmStock("Kodak Ektar 100",         "kodak_ektar_100.json",         "kodak_endura_premier.json"),
    FilmStock("Kodak Gold 200",          "kodak_gold_200.json",          "kodak_endura_premier.json"),
    FilmStock("Kodak Ultramax 400",      "kodak_ultramax_400.json",      "kodak_ultra_endura.json"),
    FilmStock("Kodak Verita 200D",       "kodak_verita_200d.json",       "kodak_endura_premier.json"),
    // ── Fujifilm Color Negative ────────────────────────────────────────────
    FilmStock("Fuji C200",              "fujifilm_c200.json",            "fujifilm_crystal_archive_typeii.json"),
    FilmStock("Fuji Superia X-TRA 400", "fujifilm_xtra_400.json",       "fujifilm_crystal_archive_typeii.json"),
    FilmStock("Fuji Pro 400H",          "fujifilm_pro_400h.json",        "fujifilm_crystal_archive_typeii.json"),
    // ── Slide / Reversal ──────────────────────────────────────────────────
    FilmStock("Fuji Velvia 100",         "fujifilm_velvia_100.json",     "fujifilm_crystal_archive_typeii.json"),
    FilmStock("Fuji Provia 100F",        "fujifilm_provia_100f.json",    "fujifilm_crystal_archive_typeii.json"),
    FilmStock("Kodak Ektachrome 100",    "kodak_ektachrome_100.json",    "kodak_endura_premier.json"),
    FilmStock("Kodak Kodachrome 64",     "kodak_kodachrome_64.json",     "kodak_endura_premier.json"),
    // ── Cine ──────────────────────────────────────────────────────────────
    // Cinestill = Vision3 with remjet removed for C-41 processing; same spectral profile
    FilmStock("Cinestill 50D",           "kodak_vision3_50d.json",       "kodak_2383.json"),
    FilmStock("Cinestill 800T",          "kodak_vision3_500t.json",      "kodak_2383.json"),
    FilmStock("Kodak Vision3 200T",      "kodak_vision3_200t.json",      "kodak_2383.json"),
    FilmStock("Kodak Vision3 250D",      "kodak_vision3_250d.json",      "kodak_2383.json"),
    FilmStock("Kodak Vision3 500T",      "kodak_vision3_500t.json",      "kodak_2383.json"),
)

class FilmProcessor(private val context: Context) {

    companion object {
        private const val TAG = "FilmProcessor"
    }

    private var currentEngine: SpectralEngine? = null
    private var currentStockIndex: Int = -1

    fun loadFilmStock(stockIndex: Int) {
        if (stockIndex == currentStockIndex && currentEngine != null) return
        currentEngine?.close()
        val stock = FILM_STOCKS[stockIndex]
        try {
            val filmJson = context.assets.open("profiles/${stock.filmProfile}").bufferedReader().readText()
            val printJson = context.assets.open("profiles/${stock.printProfile}").bufferedReader().readText()
            currentEngine = SpectralEngine(filmJson, printJson)
            currentStockIndex = stockIndex
            Log.i(TAG, "Loaded authentic film stock: ${stock.displayName}")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load film stock", e)
        }
    }

    /**
     * Process a [RawJob] produced by [com.example.latent.camera.Camera2Controller].
     * [onProgress] is called with values in [0, 1] as processing advances through stages.
     */
    suspend fun processRawJob(
        job: RawJob,
        onProgress: (Float) -> Unit = {},
    ): String? = withContext(Dispatchers.Default) {
        onProgress(0.02f)
        loadFilmStock(job.filmStockIndex)
        val engine = currentEngine ?: return@withContext null

        Log.i(TAG, "Starting Full-Res Spectral Development (${job.width}x${job.height})")

        // ── Step 1: Bilinear Demosaic & Linearization (5 % → 30 %) ─────────
        onProgress(0.05f)
        val linearRgb = bayerToFullResLinearRgb(job.pixels, job.width, job.height, job.result, job.characteristics) { rowFrac ->
            onProgress(0.05f + rowFrac * 0.25f)
        }

        // ── Step 2: Rust Spectral Engine (30 % → 85 %) ──────────────────────
        onProgress(0.30f)
        Log.i(TAG, "Calling Rust spectral engine (${job.width}x${job.height})")
        val processed = engine.process(linearRgb, job.width, job.height)
        Log.i(TAG, "Spectral engine returned: ${processed.width}x${processed.height}")

        // ── Step 3: Floats to Bitmap (85 % → 95 %) ──────────────────────────
        onProgress(0.85f)
        val outPixels = IntArray(processed.width * processed.height)
        for (i in outPixels.indices) {
            val r = linearToSrgb(processed.data[i * 3 + 0])
            val g = linearToSrgb(processed.data[i * 3 + 1])
            val b = linearToSrgb(processed.data[i * 3 + 2])
            outPixels[i] = (0xFF shl 24) or (r shl 16) or (g shl 8) or b
        }
        val bitmap = Bitmap.createBitmap(processed.width, processed.height, Bitmap.Config.ARGB_8888)
        bitmap.setPixels(outPixels, 0, processed.width, 0, 0, processed.width, processed.height)

        // ── Step 4: Save & Inject Metadata (95 % → 100 %) ──────────────────
        onProgress(0.95f)
        val stock = FILM_STOCKS[job.filmStockIndex]
        val timestamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.US).format(Date())
        val filename = "LATENT_${stock.displayName.replace(" ", "_")}_${timestamp}.jpg"
        val uri = saveWithMetadata(bitmap, filename, stock, job.result, job.characteristics)
        bitmap.recycle()

        onProgress(1.0f)
        uri
    }

    suspend fun processRaw(
        rawImage: Image,
        result: CaptureResult,
        chars: CameraCharacteristics,
        filmStockIndex: Int
    ): String? {
        val plane = rawImage.planes[0]
        val shortBuf = plane.buffer.asShortBuffer()
        val pixels = ShortArray(shortBuf.remaining())
        shortBuf.get(pixels)
        val job = RawJob(pixels, rawImage.width, rawImage.height,
            result as TotalCaptureResult, chars, filmStockIndex)
        return processRawJob(job)
    }

    private fun bayerToFullResLinearRgb(
        pixels: ShortArray,
        width: Int,
        height: Int,
        result: CaptureResult,
        chars: CameraCharacteristics,
        rowProgress: ((Float) -> Unit)? = null,
    ): FloatArray {
        val blackLevel = chars.get(CameraCharacteristics.SENSOR_BLACK_LEVEL_PATTERN)
        val whiteLevel = chars.get(CameraCharacteristics.SENSOR_INFO_WHITE_LEVEL) ?: 1023

        val output = FloatArray(width * height * 3)

        for (y in 1 until height - 1) {
            if (rowProgress != null && y % 100 == 0) rowProgress(y.toFloat() / height)
            for (x in 1 until width - 1) {
                val idx = y * width + x
                val p = pixels[idx].toInt() and 0xFFFF

                val r: Float
                val g: Float
                val b: Float

                if (y % 2 == 0) {
                    if (x % 2 == 0) { // Red pixel
                        r = normalize(p, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel)
                        g = (normalize(pixels[idx-1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 0) ?: 0, whiteLevel) +
                             normalize(pixels[idx+1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 0) ?: 0, whiteLevel) +
                             normalize(pixels[idx-width].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 1) ?: 0, whiteLevel) +
                             normalize(pixels[idx+width].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 1) ?: 0, whiteLevel)) / 4f
                        b = (normalize(pixels[idx-width-1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel) +
                             normalize(pixels[idx-width+1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel) +
                             normalize(pixels[idx+width-1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel) +
                             normalize(pixels[idx+width+1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel)) / 4f
                    } else { // Green pixel (row 0)
                        g = normalize(p, blackLevel?.getOffsetForIndex(1, 0) ?: 0, whiteLevel)
                        r = (normalize(pixels[idx-1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel) +
                             normalize(pixels[idx+1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel)) / 2f
                        b = (normalize(pixels[idx-width].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel) +
                             normalize(pixels[idx+width].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel)) / 2f
                    }
                } else {
                    if (x % 2 == 0) { // Green pixel (row 1)
                        g = normalize(p, blackLevel?.getOffsetForIndex(0, 1) ?: 0, whiteLevel)
                        r = (normalize(pixels[idx-width].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel) +
                             normalize(pixels[idx+width].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel)) / 2f
                        b = (normalize(pixels[idx-1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel) +
                             normalize(pixels[idx+1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel)) / 2f
                    } else { // Blue pixel
                        b = normalize(p, blackLevel?.getOffsetForIndex(1, 1) ?: 0, whiteLevel)
                        g = (normalize(pixels[idx-1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 1) ?: 0, whiteLevel) +
                             normalize(pixels[idx+1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 1) ?: 0, whiteLevel) +
                             normalize(pixels[idx-width].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 0) ?: 0, whiteLevel) +
                             normalize(pixels[idx+width].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(1, 0) ?: 0, whiteLevel)) / 4f
                        r = (normalize(pixels[idx-width-1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel) +
                             normalize(pixels[idx-width+1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel) +
                             normalize(pixels[idx+width-1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel) +
                             normalize(pixels[idx+width+1].toInt() and 0xFFFF, blackLevel?.getOffsetForIndex(0, 0) ?: 0, whiteLevel)) / 4f
                    }
                }

                val outIdx = idx * 3
                output[outIdx] = r
                output[outIdx + 1] = g
                output[outIdx + 2] = b
            }
        }
        return output
    }

    private fun normalize(value: Int, black: Int, white: Int): Float {
        return ((value - black).toFloat() / (white - black).toFloat()).coerceIn(0f, 1f)
    }

    private fun linearToSrgb(c: Float): Int {
        val v = if (c <= 0.0031308f) c * 12.92f
        else (1.055f * Math.pow(c.toDouble(), 1.0 / 2.4).toFloat() - 0.055f)
        return (v * 255f).toInt().coerceIn(0, 255)
    }

    private fun saveWithMetadata(
        bitmap: Bitmap,
        filename: String,
        stock: FilmStock,
        result: CaptureResult,
        chars: CameraCharacteristics
    ): String? {
        val contentValues = ContentValues().apply {
            put(MediaStore.Images.Media.DISPLAY_NAME, filename)
            put(MediaStore.Images.Media.MIME_TYPE, "image/jpeg")
            put(MediaStore.Images.Media.RELATIVE_PATH, "Pictures/Latent/Developed")
            put(MediaStore.Images.Media.IS_PENDING, 1)
        }
        val resolver = context.contentResolver
        val uri = resolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, contentValues) ?: return null
        
        resolver.openFileDescriptor(uri, "rw")?.use { fd ->
            java.io.FileOutputStream(fd.fileDescriptor).use { os ->
                bitmap.compress(Bitmap.CompressFormat.JPEG, 98, os)
            }
            
            // Inject EXIF Metadata
            val exif = ExifInterface(fd.fileDescriptor)
            exif.setAttribute(ExifInterface.TAG_MAKE, "Latent Cam")
            exif.setAttribute(ExifInterface.TAG_MODEL, stock.displayName)
            exif.setAttribute(ExifInterface.TAG_SOFTWARE, "Spectral Film Engine 2.0")
            
            // Focal length
            val focal = result.get(CaptureResult.LENS_FOCAL_LENGTH) ?: 0f
            exif.setAttribute(ExifInterface.TAG_FOCAL_LENGTH, focal.toString())
            
            exif.saveAttributes()
        }

        contentValues.clear()
        contentValues.put(MediaStore.Images.Media.IS_PENDING, 0)
        resolver.update(uri, contentValues, null, null)
        
        Log.i(TAG, "Developed authentic photo with metadata: $filename")
        return uri.toString()
    }

    suspend fun generatePreviewLut(stockIndex: Int): FloatArray? = withContext(Dispatchers.Default) {
        loadFilmStock(stockIndex)
        currentEngine?.generateLut(33)
    }

    suspend fun updateExposure(ev: Float) = withContext(Dispatchers.Default) {
        currentEngine?.softUpdateExposurePrintFilters(exposureCompensationEv = ev.toDouble())
    }

    fun close() {
        currentEngine?.close()
    }
}
