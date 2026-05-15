package com.example.latent.processing

import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.TotalCaptureResult

data class RawJob(
    val pixels: ShortArray,
    val width: Int,
    val height: Int,
    val result: TotalCaptureResult,
    val characteristics: CameraCharacteristics,
    val filmStockIndex: Int,
) {
    // ShortArray doesn't implement equals/hashCode by value; add manual implementations.
    override fun equals(other: Any?) = other is RawJob && pixels.contentEquals(other.pixels)
    override fun hashCode() = pixels.contentHashCode()
}
