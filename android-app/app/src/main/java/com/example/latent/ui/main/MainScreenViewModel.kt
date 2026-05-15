package com.example.latent.ui.main

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

data class CameraUiState(
    val selectedFilmIndex: Int = 0,
    val exposureCompEv: Float = 0f,
    val selectedLensLabel: String = "24mm",
    val isCapturing: Boolean = false,
    val frameCount: Int = 24,
)

class MainScreenViewModel : ViewModel() {
    private val _uiState = MutableStateFlow(CameraUiState())
    val uiState: StateFlow<CameraUiState> = _uiState.asStateFlow()
}
