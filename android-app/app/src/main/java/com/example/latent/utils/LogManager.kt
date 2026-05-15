package com.example.latent.utils

import android.content.ContentValues
import android.content.Context
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.util.Log
import java.io.File
import java.io.FileOutputStream
import java.io.InputStreamReader
import java.text.SimpleDateFormat
import java.util.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Programmatically captures Logcat output (including Rust NDK logs).
 */
object LogManager {
    private const val TAG = "LogManager"
    private var isLoggingActive = false

    fun setLoggingActive(active: Boolean) {
        isLoggingActive = active
        Log.i(TAG, "Debug logging ${if (active) "ENABLED" else "DISABLED"}")
    }

    fun isLoggingActive() = isLoggingActive

    /**
     * Captures the current logcat buffer and saves it to a file.
     *
     * On API 29+ (Android 10+) uses MediaStore.Downloads so no storage permission
     * is required and legacy File I/O restrictions are avoided.
     * On API 24–28 falls back to the legacy File path (covered by the manifest
     * WRITE_EXTERNAL_STORAGE permission with maxSdkVersion=28).
     *
     * @return A string identifying the saved file (content URI on API 29+,
     *         absolute path on API < 29), or null if the export failed.
     */
    suspend fun exportLogs(context: Context): String? = withContext(Dispatchers.IO) {
        val timestamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.US).format(Date())
        val fileName = "LATENT_LOG_$timestamp.txt"

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            // --- API 29+ path: MediaStore.Downloads (no permission needed) ---
            val contentValues = ContentValues().apply {
                put(MediaStore.Downloads.DISPLAY_NAME, fileName)
                put(MediaStore.Downloads.MIME_TYPE, "text/plain")
                put(MediaStore.Downloads.RELATIVE_PATH, "Download/Latent/Logs")
                put(MediaStore.Downloads.IS_PENDING, 1)
            }

            val uri = context.contentResolver.insert(
                MediaStore.Downloads.EXTERNAL_CONTENT_URI,
                contentValues
            ) ?: return@withContext null

            try {
                // "logcat -d" dumps the current log buffer and exits
                val process = Runtime.getRuntime().exec("logcat -d")
                val reader = InputStreamReader(process.inputStream)

                context.contentResolver.openOutputStream(uri)?.use { output ->
                    output.write("--- LATENT CAM DEBUG LOG ---\n".toByteArray())
                    output.write("Device: ${Build.MODEL}\n".toByteArray())
                    output.write("Time: ${Date()}\n\n".toByteArray())

                    val buffer = CharArray(4096)
                    var read: Int
                    while (reader.read(buffer).also { read = it } != -1) {
                        output.write(String(buffer, 0, read).toByteArray())
                    }
                }

                // Mark the entry as complete so it is visible to other apps
                val updateValues = ContentValues().apply {
                    put(MediaStore.Downloads.IS_PENDING, 0)
                }
                context.contentResolver.update(uri, updateValues, null, null)

                Log.i(TAG, "Logs exported to: $uri")
                uri.toString()
            } catch (e: Exception) {
                Log.e(TAG, "Failed to export logs", e)
                // Remove the incomplete MediaStore entry on failure
                context.contentResolver.delete(uri, null, null)
                null
            }
        } else {
            // --- API 24–28 path: legacy File I/O ---
            val logDir = File(
                Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS),
                "Latent/Logs"
            )
            if (!logDir.exists()) logDir.mkdirs()

            val logFile = File(logDir, fileName)

            try {
                // "logcat -d" dumps the current log buffer and exits
                val process = Runtime.getRuntime().exec("logcat -d")
                val reader = InputStreamReader(process.inputStream)

                FileOutputStream(logFile).use { output ->
                    output.write("--- LATENT CAM DEBUG LOG ---\n".toByteArray())
                    output.write("Device: ${Build.MODEL}\n".toByteArray())
                    output.write("Time: ${Date()}\n\n".toByteArray())

                    val buffer = CharArray(4096)
                    var read: Int
                    while (reader.read(buffer).also { read = it } != -1) {
                        output.write(String(buffer, 0, read).toByteArray())
                    }
                }

                Log.i(TAG, "Logs exported to: ${logFile.absolutePath}")
                logFile.absolutePath
            } catch (e: Exception) {
                Log.e(TAG, "Failed to export logs", e)
                null
            }
        }
    }
}
