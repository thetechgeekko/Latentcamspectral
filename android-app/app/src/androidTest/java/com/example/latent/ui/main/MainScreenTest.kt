package com.example.latent.ui.main

import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import com.example.latent.MainActivity
import org.junit.Test
import org.junit.runner.RunWith

/** Smoke test: verifies MainActivity launches without crashing. */
@RunWith(AndroidJUnit4::class)
class MainActivitySmokeTest {

    @Test
    fun mainActivity_launchesWithoutCrash() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            // If we reach here without an exception the activity started successfully.
            assert(scenario != null)
        }
    }
}
