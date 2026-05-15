# JNI Expose for LUT Baking Complete

The Rust 3D LUT generator has now been fully exposed to the Android Kotlin layer.

## Key Changes

1. **Native C-ABI Endpoint (`spectralfilm-ndk/src/lib.rs`)**:
   - Added `generate_lut_f32()` to the safe `Engine` struct.
   - Exported `Java_com_latentcam_spectralfilm_SpectralEngine_nativeGenerateLut` to interface with the JVM. It securely allocates a `jfloatArray` and copies the baked 3D RGB lattice memory into it.

2. **Kotlin API (`SpectralEngine.kt`)**:
   - Declared the `nativeGenerateLut` external function.
   - Added a robust public wrapper:
     ```kotlin
     /** Generates a baked 3D LUT (Look-Up Table) of the current spectral color transformation. */
     fun generateLut(lutSize: Int = 33): FloatArray {
         check(nativeHandle != 0L) { "SpectralEngine has been closed" }
         require(lutSize in 2..64) { "lutSize must be between 2 and 64" }
         return nativeGenerateLut(nativeHandle, lutSize)
     }
     ```

## How to use it in the Android App

When you build your OpenGL preview shader, you just call this once when the user selects a film stock:

```kotlin
val lutData = spectralEngine.generateLut(33) // Returns FloatArray of size 35,937 (33x33x33 RGB)
// Upload lutData to a GL_TEXTURE_3D in your OpenGL engine
```

Every time the user swipes to a new film, or adjusts the Exposure Compensation slider, just call `generateLut(33)` again and update the 3D texture!
