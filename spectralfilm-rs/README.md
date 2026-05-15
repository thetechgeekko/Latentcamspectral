# spectralfilm-rs Android Library

This directory contains a Rust/Android port of the Python `spektrafilm` processing engine.

It is split into two crates:

- `spectralfilm-core` — pure Rust simulation core: profiles, spectral math, film development, DIR couplers, grain, halation, printing, scanning, resize/crop, timings.
- `spectralfilm-ndk` — Android JNI/C ABI bridge exposing `libspectralfilm_ndk.so`.

The original Python profile/filter/LUT data has been copied to `spectralfilm-core/assets/data` so Android apps can package it as assets or embed selected profiles as strings.

## Status

This is a full native-library scaffold with the end-to-end pipeline ported into Rust. The implementation keeps the original runtime concepts and parameters while using Android-friendly memory ownership and float buffers.

Important note: this repository environment does not currently have `rustc`, `cargo`, or `rustup` installed, so I could not run `cargo check` locally here. The code is written as a normal Cargo workspace and is ready for validation once Rust and the Android NDK are available.

## Install toolchain

1. Install Rust from <https://rustup.rs>
2. Install Android Studio / Android NDK
3. Install cargo-ndk:

```text
cargo install cargo-ndk
```

4. Add Android targets:

```text
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

## Build desktop check

From `spectralfilm-rs`:

```text
cargo check --workspace
cargo test --workspace
```

## Build Android `.so` libraries

From `spectralfilm-rs`:

```text
cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -o ../android/app/src/main/jniLibs \
  build -p spectralfilm-ndk --release
```

This produces Android ABI folders containing `libspectralfilm_ndk.so`.

## Kotlin usage

Copy `android-kotlin/com/latentcam/spectralfilm/SpectralEngine.kt` into your Android app source tree.

Package profile JSON files into your app assets, for example:

- `assets/profiles/kodak_portra_400.json`
- `assets/profiles/kodak_portra_endura.json`

Then:

```text
val filmJson = assets.open("profiles/kodak_portra_400.json").bufferedReader().readText()
val printJson = assets.open("profiles/kodak_portra_endura.json").bufferedReader().readText()

SpectralEngine(filmJson, printJson).use { engine ->
    val processed = engine.process(linearRgbFloatArray, width, height)
    val outRgb = processed.data
}
```

Input buffer requirements:

- Interleaved RGB `FloatArray`
- Length at least `width * height * 3`
- Linear, scene-referred RGB; default interpretation is ProPhoto RGB, matching the Python reference defaults
- Values are expected to be finite and non-negative

Output:

- Interleaved RGB `FloatArray`
- Dimensions may differ from input if crop/upscale params are enabled
- Default output is sRGB-encoded and clipped to `[0, 1]`

## C ABI usage

A C header is provided at `include/spectralfilm.h`.

The minimal native API is:

- `spectralfilm_create_from_json(film_json, print_json)`
- `spectralfilm_process_f32(engine, input, width, height, output, output_capacity)`
- `spectralfilm_last_width(engine)`
- `spectralfilm_last_height(engine)`
- `spectralfilm_destroy(engine)`

## Current feature coverage

Ported from Python:

- Profile schema and JSON loading, including nested numeric tensors and `null` → `NaN`
- Runtime parameter hierarchy
- Profile-specific digest rules
- RGB → film raw exposure with Hanatos-compatible fallback and Mallett-compatible fallback
- UV/IR bandpass filters
- Camera diffusion filter
- Lens blur
- Halation and highlight boost
- H&D characteristic-curve interpolation
- DIR coupler matrix and spatial diffusion
- Grain with sublayer support and deterministic stochastic sampling
- Printing through filtered enlarger light
- Print preflash
- Print development
- Scanning through spectral density → XYZ → RGB
- Glare, scanner blur, unsharp mask
- Crop/resize and physical pixel-size tracking
- Timing instrumentation
- JNI and C ABI bridge

## Future performance hooks

The structure is ready for these next optimizations without changing public API:

- Replace the fallback spectral upsampler with the original binary Hanatos `irradiance_xy_tc.npy` LUT converted to a Rust asset.
- Add cached 3D LUTs for enlarger/scanner spectral calculations (`utils::lut::Lut3d` is already implemented).
- Add Vulkan compute kernels for large Gaussian/halation passes on Android GPU.
- Replace `f64` inner buffers with SIMD-optimized `f32` kernels where visual parity permits.
