# Zero-Copy Memory Optimization Complete

The Rust engine now natively supports **Direct Java NIO ByteBuffers** via JNI, eliminating massive memory spikes when taking high-resolution photos!

## What Changed?

1. **JNI Integration (`spectralfilm-ndk`)**:
   - Added `nativeProcessDirect`, a C-ABI endpoint that bypasses standard JVM array copying.
   - It uses `env.get_direct_buffer_address` to obtain direct C pointers to memory allocated by the Android OS. 
   - Uses `unsafe` Rust pointer casting (`std::slice::from_raw_parts`) to treat Android's memory exactly like a Rust `&[f32]` slice.

2. **Kotlin API (`SpectralEngine.kt`)**:
   - Added `processDirect(inputRgb: ByteBuffer, outputRgb: ByteBuffer, width: Int, height: Int)`.

## How to use it in the Android App

When you capture a RAW image in Android, allocate two direct buffers. The JVM won't manage this memory (meaning no garbage collection pauses!), and Rust will process the photo directly inside these buffers.

```kotlin
// 1. Allocate zero-copy Direct buffers (e.g. for a 12MP photo)
val floatCount = width * height * 3
val inputBuffer = ByteBuffer.allocateDirect(floatCount * 4).order(ByteOrder.nativeOrder())
val outputBuffer = ByteBuffer.allocateDirect(floatCount * 4).order(ByteOrder.nativeOrder())

// 2. Put your RAW image data into `inputBuffer`

// 3. Process! (Rust reads from inputBuffer and writes to outputBuffer instantly)
engine.processDirect(inputBuffer, outputBuffer, width, height)

// 4. The `outputBuffer` now contains the fully developed spectral film image!
```

> [!TIP]
> This optimization drops peak RAM usage by hundreds of megabytes during high-res photo captures, ensuring your app won't crash on older Android devices with limited memory.
