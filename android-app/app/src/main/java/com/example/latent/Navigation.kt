package com.example.latent

import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.navigation3.runtime.entryProvider
import androidx.navigation3.runtime.rememberNavBackStack
import androidx.navigation3.ui.NavDisplay
import com.example.latent.ui.main.CameraScreen
import com.example.latent.ui.gallery.GalleryScreen

@Composable
fun MainNavigation() {
    val backStack = rememberNavBackStack(Main)

    NavDisplay(
        backStack = backStack,
        onBack = { backStack.removeLastOrNull() },
        entryProvider =
            entryProvider {
                entry<Main> {
                    CameraScreen(
                        modifier = Modifier.safeDrawingPadding(),
                        onGalleryOpen = { backStack.add(Gallery) },
                    )
                }
                entry<Gallery> {
                    GalleryScreen(onBack = { backStack.removeLastOrNull() })
                }
            },
    )
}
