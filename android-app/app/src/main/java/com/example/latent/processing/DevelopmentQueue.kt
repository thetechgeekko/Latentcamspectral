package com.example.latent.processing

import android.content.Context
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class DevelopmentStatus(
    val queueSize: Int = 0,
    val progress: Float = 0f,
    val isProcessing: Boolean = false,
)

object DevelopmentQueue {

    val jobs = Channel<RawJob>(Channel.UNLIMITED)

    private val _status = MutableStateFlow(DevelopmentStatus())
    val status: StateFlow<DevelopmentStatus> = _status.asStateFlow()

    fun enqueue(job: RawJob, context: Context) {
        jobs.trySend(job)
        _status.update { it.copy(queueSize = it.queueSize + 1) }
        DevelopmentService.start(context)
    }

    fun reportProgress(progress: Float) {
        _status.update { it.copy(progress = progress, isProcessing = true) }
    }

    fun reportJobComplete() {
        _status.update { it.copy(queueSize = (it.queueSize - 1).coerceAtLeast(0)) }
    }

    fun reportIdle() {
        _status.update { DevelopmentStatus() }
    }
}
