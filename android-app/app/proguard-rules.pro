# Keep JNI-accessible native method names
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep the SpectralEngine JNI bridge intact
-keep class com.latentcam.spectralfilm.** { *; }

# Keep Kotlin coroutine infrastructure
-keepnames class kotlinx.coroutines.internal.MainDispatcherFactory {}
-keepnames class kotlinx.coroutines.CoroutineExceptionHandler {}

# Compose
-keep class androidx.compose.** { *; }
-dontwarn androidx.compose.**
