package com.example.latent.processing

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*

class DevelopmentService : Service() {

    companion object {
        private const val TAG = "DevelopmentService"
        private const val CHANNEL_ID = "latent_darkroom"
        private const val NOTIF_ID = 1001

        fun start(context: Context) {
            val intent = Intent(context, DevelopmentService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }
    }

    private val serviceScope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private var processingJob: Job? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val notification = buildNotification("Darkroom starting…")
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIF_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            startForeground(NOTIF_ID, notification)
        }
        if (processingJob?.isActive != true) {
            processingJob = serviceScope.launch { processLoop() }
        }
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        super.onDestroy()
        serviceScope.cancel()
    }

    private suspend fun processLoop() {
        val processor = FilmProcessor(this)
        try {
            while (true) {
                // Wait up to 3 s for the next job; stop the service when queue drains.
                val job = withTimeoutOrNull(3_000L) { DevelopmentQueue.jobs.receive() } ?: break

                DevelopmentQueue.reportProgress(0f)

                processor.processRawJob(job) { progress ->
                    DevelopmentQueue.reportProgress(progress)
                    val remaining = DevelopmentQueue.status.value.queueSize
                    val pct = (progress * 100).toInt()
                    updateNotification("Developing $remaining photo(s) · $pct%", pct)
                }

                DevelopmentQueue.reportJobComplete()
                Log.i(TAG, "Development job complete")
            }
        } catch (e: Exception) {
            Log.e(TAG, "processLoop error", e)
        } finally {
            processor.close()
            DevelopmentQueue.reportIdle()
            stopSelf()
        }
    }

    private fun updateNotification(text: String, progress: Int) {
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIF_ID, buildNotification(text, progress))
    }

    private fun buildNotification(text: String, progress: Int = -1): Notification {
        val builder = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_menu_camera)
            .setContentTitle("Darkroom")
            .setContentText(text)
            .setOngoing(true)
            .setSilent(true)
            .setForegroundServiceBehavior(NotificationCompat.FOREGROUND_SERVICE_IMMEDIATE)
        if (progress >= 0) {
            builder.setProgress(100, progress, false)
        }
        return builder.build()
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Darkroom",
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = "Film development progress"
            setShowBadge(false)
        }
        val nm = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.createNotificationChannel(channel)
    }
}
