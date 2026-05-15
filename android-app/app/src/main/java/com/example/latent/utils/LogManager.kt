package com.example.latent.utils

import android.content.Context
import android.os.Environment
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
     * @return The absolute path to the log file, or null if failed.
     */
    suspend fun exportLogs(context: Context): String? = withContext(Dispatchers.IO) {
        val timestamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.US).format(Date())
        val fileName = "LATENT_LOG_$timestamp.txt"
        
        // Save to public Downloads/Latent/Logs
        val logDir = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), "Latent/Logs")
        if (!logDir.exists()) logDir.mkdirs()
        
        val logFile = File(logDir, fileName)

        try {
            // "logcat -d" dumps the current log buffer and exits
            val process = Runtime.getRuntime().exec("logcat -d")
            val reader = InputStreamReader(process.inputStream)
            val writer = FileOutputStream(logFile)

            writer.use { output ->
                output.write("--- LATENT CAM DEBUG LOG ---\n".toByteArray())
                output.write("Device: ${android.os.Build.MODEL}\n".toByteArray())
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
