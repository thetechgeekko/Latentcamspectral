package com.latentcam.spectralfilm

import java.nio.ByteBuffer

/**
 * Android/Kotlin wrapper around the Rust `spectralfilm_ndk` library.
 *
 * Pass profile JSON strings from assets, e.g.:
 * ```kotlin
 * val filmJson = assets.open("profiles/kodak_portra_400.json").bufferedReader().readText()
 * val printJson = assets.open("profiles/kodak_portra_endura.json").bufferedReader().readText()
 * val engine = SpectralEngine(filmJson, printJson)
 * val out = engine.process(linearRgb, width, height)
 * ```
 */
class SpectralEngine : AutoCloseable {
    private var nativeHandle: Long = 0L

    constructor(filmProfileJson: String, printProfileJson: String) {
        nativeInitLogger()
        nativeHandle = nativeCreate(filmProfileJson, printProfileJson)
        require(nativeHandle != 0L) { "Failed to create native SpectralEngine" }
    }

    constructor(paramsJson: String) {
        nativeInitLogger()
        nativeHandle = nativeCreateFromParamsJson(paramsJson)
        require(nativeHandle != 0L) { "Failed to create native SpectralEngine from runtime params JSON" }
    }

    /** Process interleaved linear RGB float data. Input length must be at least width*height*3. */
    fun process(inputLinearRgb: FloatArray, width: Int, height: Int): ProcessedImage {
        check(nativeHandle != 0L) { "SpectralEngine has been closed" }
        require(width > 0 && height > 0) { "width and height must be positive" }
        require(inputLinearRgb.size >= width * height * 3) { "input buffer must contain width*height*3 floats" }
        val data = nativeProcess(nativeHandle, inputLinearRgb, width, height)
        return ProcessedImage(
            data = data,
            width = nativeGetLastWidth(nativeHandle),
            height = nativeGetLastHeight(nativeHandle),
            channels = nativeGetLastChannels(nativeHandle),
        )
    }

    /** Process interleaved linear RGB float data directly between memory-mapped NIO buffers (Zero-Copy). */
    fun processDirect(inputRgb: ByteBuffer, outputRgb: ByteBuffer, width: Int, height: Int): Boolean {
        check(nativeHandle != 0L) { "SpectralEngine has been closed" }
        require(width > 0 && height > 0) { "width and height must be positive" }
        return nativeProcessDirect(nativeHandle, inputRgb, outputRgb, width, height)
    }

    /** Generates a baked 3D LUT (Look-Up Table) of the current spectral color transformation. */
    fun generateLut(lutSize: Int = 33): FloatArray {
        check(nativeHandle != 0L) { "SpectralEngine has been closed" }
        require(lutSize in 2..64) { "lutSize must be between 2 and 64" }
        return nativeGenerateLut(nativeHandle, lutSize)
    }

    /** Replace all runtime parameters with a serialized RuntimePhotoParams JSON object. */
    fun updateParamsJson(paramsJson: String): Boolean {
        check(nativeHandle != 0L) { "SpectralEngine has been closed" }
        return nativeUpdateParamsJson(nativeHandle, paramsJson)
    }

    /** Apply a partial RuntimeParamsPatch JSON object and rebuild the native pipeline. */
    fun softUpdateParamsJson(patchJson: String): Boolean {
        check(nativeHandle != 0L) { "SpectralEngine has been closed" }
        return nativeSoftUpdateParamsJson(nativeHandle, patchJson)
    }

    /**
     * Convenience soft update for interactive controls.
     * Null values leave the current native parameter unchanged.
     */
    fun softUpdateExposurePrintFilters(
        exposureCompensationEv: Double? = null,
        autoExposure: Boolean? = null,
        printExposure: Double? = null,
        yFilterShift: Double? = null,
        mFilterShift: Double? = null,
        cFilterNeutral: Double? = null,
        yFilterNeutral: Double? = null,
        mFilterNeutral: Double? = null,
    ): Boolean {
        check(nativeHandle != 0L) { "SpectralEngine has been closed" }
        val autoExposureMode = when (autoExposure) {
            null -> -1
            false -> 0
            true -> 1
        }
        return nativeSoftUpdateExposurePrintFilters(
            nativeHandle,
            exposureCompensationEv ?: Double.NaN,
            autoExposureMode,
            printExposure ?: Double.NaN,
            yFilterShift ?: Double.NaN,
            mFilterShift ?: Double.NaN,
            cFilterNeutral ?: Double.NaN,
            yFilterNeutral ?: Double.NaN,
            mFilterNeutral ?: Double.NaN,
        )
    }

    /** Current serialized native runtime params, or an empty string after close. */
    fun paramsJson(): String = if (nativeHandle != 0L) nativeGetParamsJson(nativeHandle) else ""

    fun timings(): String = if (nativeHandle != 0L) nativeGetTimings(nativeHandle) else ""

    override fun close() {
        if (nativeHandle != 0L) {
            nativeRelease(nativeHandle)
            nativeHandle = 0L
        }
    }

    protected fun finalize() {
        close()
    }

    data class ProcessedImage(
        val data: FloatArray,
        val width: Int,
        val height: Int,
        val channels: Int,
    )

    private external fun nativeInitLogger()
    private external fun nativeCreate(filmJson: String, printJson: String): Long
    private external fun nativeCreateFromParamsJson(paramsJson: String): Long
    private external fun nativeRelease(handle: Long)
    private external fun nativeProcess(handle: Long, input: FloatArray, width: Int, height: Int): FloatArray
    private external fun nativeProcessDirect(handle: Long, inputBuf: ByteBuffer, outputBuf: ByteBuffer, width: Int, height: Int): Boolean
    private external fun nativeGenerateLut(handle: Long, lutSize: Int): FloatArray
    private external fun nativeUpdateParamsJson(handle: Long, paramsJson: String): Boolean
    private external fun nativeSoftUpdateParamsJson(handle: Long, patchJson: String): Boolean
    private external fun nativeSoftUpdateExposurePrintFilters(
        handle: Long,
        exposureCompensationEv: Double,
        autoExposureMode: Int,
        printExposure: Double,
        yFilterShift: Double,
        mFilterShift: Double,
        cFilterNeutral: Double,
        yFilterNeutral: Double,
        mFilterNeutral: Double,
    ): Boolean
    private external fun nativeGetParamsJson(handle: Long): String
    private external fun nativeGetLastWidth(handle: Long): Int
    private external fun nativeGetLastHeight(handle: Long): Int
    private external fun nativeGetLastChannels(handle: Long): Int
    private external fun nativeGetTimings(handle: Long): String

    companion object {
        init {
            System.loadLibrary("spectralfilm_ndk")
        }
    }
}
