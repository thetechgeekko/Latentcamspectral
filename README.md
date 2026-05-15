# spectralfilm-rs: Android Processing Engine

> [!IMPORTANT]
> At this stage, this project is very experimental and a work in progress. Things might change fast. This repository contains **only the headless processing engine**, rewritten in Rust and optimized for Android via JNI/NDK.

An exploration of how to make good use of spectroscopic data from manufacturer datasheets in an end-to-end, physically based model with spectral calculations, with the goal of turning that data into convincing film, print, and scan renderings natively on mobile devices.

An high-level writeup and discussion on the original desktop project is available on [discuss.pixls.us]().

In practice, `spectralfilm-rs` provides a high-performance computational photography backend. It lets you take linear RAW data directly from the Android camera sensor, pass it through a virtual negative, print, and scan pipeline, and calculate how film-stock data, couplers, grain, and halation shape the final result. The aim is not just to imitate a generic "film look," but to build a model that stays connected to the structure and behavior of real photographic materials, all running within a strict mobile memory and performance budget.

## Introduction

The simulation emulates negative or positive film emulsions starting from published data for film stocks. An example of the curves for Kodak Portra 400 (data-sheet e4050, 2016) is shown in the following figure (note that the CMY diffuse densities are generic because they are usually not published).

An example of data for Kodak Portra Endura print paper (data-sheet e4021, 2009) is shown in the next figure.

The left panel shows the spectral log sensitivities of each color layer. The central panel shows the log-exposure-density characteristic curves for each layer when the medium is exposed to a neutral gray gradient under a reference light. The panel on the right shows the absorption spectra of the dyes formed on the medium during chemical development.

Starting from linear RGB data (ideally directly from a RAW buffer), the simulation reconstructs the spectral data, projects the virtual light transmitted through the negative onto print paper, and uses a simplified color enlarger with dichroic filters to balance the print.

The pipeline is sketched in this figure, adapted from [^1]:


Data-sheet curves are really not enough to reproduce a decent film look. The key is to understand that film emulsions contain couplers, chemicals that are produced during development alongside the actual CMY dyes, and these are very important for achieving the desired saturation. The main ones are:

* **Masking couplers:** Give the typical orange color to unexposed developed film. These are simulated with a negative absorption contribution in the isolated dye absorption spectra.

* **Direct inhibitor couplers:** Released locally when density is formed, inhibiting the formation of density in nearby layers. If we let the couplers diffuse in space, they increase local contrast and perceived sharpness.

## Architecture & Layout

Since this is the mobile processing port, all GUI elements have been stripped away in favor of a lean, native core. The codebase is organized into two primary crates:

1. `spectralfilm-core`: Pure Rust runtime simulation pipeline. It handles the heavy lifting: spectral mathematics, stochastic grain generation, and dye absorption calculations. We are actively migrating complex spatial operations (like halation blurring) to Vulkan compute shaders to free up the CPU.
2. `spectralfilm-ndk`: The JNI bridge exposing a C-ABI. This manages memory sharing between the JVM and Rust space, ensuring zero-copy buffer passing for large image arrays.

## Installation & Building

You will need the Android NDK and Rust installed on your machine. We recommend using `cargo-ndk` for compilation.

```bash
# Install cargo-ndk
cargo install cargo-ndk

# Add Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

# Clone the repository
git clone https://github.com/your-username/spectralfilm-rs.git
cd spectralfilm-rs

# Build the JNI libraries
cargo ndk -t arm64-v8a -t x86_64 -o ./android/app/src/main/jniLibs build --release

```

## Android Usage

The core engine is designed to sit inside a RAW-first pipeline. For best results, intercept the image data from Camera2/CameraX before the ISP applies nonlinear tone mapping or noise reduction.

Minimal Kotlin integration:

```kotlin
import com.latentcam.spectralfilm.SpectralEngine
import com.latentcam.spectralfilm.SimulationParams
import com.latentcam.spectralfilm.profiles.KodakPortra400
import com.latentcam.spectralfilm.profiles.KodakPortraEndura

// Initialize the native Rust engine
val engine = SpectralEngine()

// Configure the pipeline
val params = SimulationParams.Builder()
    .setFilmProfile(KodakPortra400())
    .setPrintProfile(KodakPortraEndura())
    .setHalation(size = 2.5f, strength = 0.15f)
    .build()

// Pass linear RAW data directly (e.g., from an ImageProxy or DngCreator buffer)
// Engine processes via NDK and returns the developed byte array
val processedBuffer = engine.processRaw(rawImageBuffer, width, height, params)

// Clean up native resources
engine.release()

```

## Testing

Testing is handled via standard Rust tooling. Regression snapshots are stored as `.npz` files (parsed natively in tests) in `tests/baselines/`.

```bash
cargo test --release

```

When a simulation change is intentional and you need to update the mathematical baselines:

```bash
cargo run --bin regenerate_test_baselines

```

## Input Data & Performance Considerations

* **Memory Management:** Processing 16-bit or 32-bit float buffers on mobile devices is memory intensive. Ensure your application requests `largeHeap="true"` in the Android Manifest, and always manually release native buffers via the provided JNI hooks to prevent out-of-memory (OOM) crashes.
* **Color Spaces:** The simulation expects linear, scene-referred data. Convert Android's sensor output into linear ProPhoto RGB or Rec2020 before passing the buffer to the Rust engine.

## References

[^1]: Giorgianni, Madden, Digital Color Management, 2nd edition, 2008 Wiley
[^2]: Hung, The Reproduction of Color, 6th edition, 2004 Wiley
[^3]: Mallett, Yuksel, Spectral Primary Decomposition for Rendering with sRGB Reflectance, Eurographics Symposium on Rendering - DL-only and Industry Track, 2019, doi:10.2312/SR.20191216
