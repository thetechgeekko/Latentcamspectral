# Post-Subagent Implementation Status

Ten implementation-only subagents were run. No tests were created or run.

## Implemented by subagents

1. Spectral upsampling
   - Added `.npy` parsing for embedded Hanatos `irradiance_xy_tc.npy`.
   - Added triangular/square coordinate helpers.
   - Added RGB → xy/b flow with adaptation helpers.
   - Added TC raw LUT generation and cubic interpolation path.
   - Added coefficient `.lut` fallback and final smooth fallback.
   - Mallett 2019 remains an approximation because exact `colour-science` basis tables are not bundled.

2. Spectral LUT service
   - Added `runtime/services/spectral_lut.rs`.
   - Added cache structures for filming TC LUT, enlarger LUT, scanner LUT.
   - Added direct fallback when LUT disabled or invalid.
   - Service still needs to be wired into runtime stages/pipeline.

3. Diffusion filters
   - Expanded `model/diffusion.rs` with family configs, scatter mapping, overrides, radial profile and PSF generation.
   - Application path still approximates some exponential components with moment-matched Gaussian convolution.

4. Color filters and illuminants
   - Added embedded CSV-backed filter support.
   - Added dichroic brand support and KG/lens filters.
   - Added TH-KG3-L support.
   - Remaining limitation: uses linear interpolation instead of SciPy Akima; some arbitrary `colour-science` illuminants remain approximations.

5. Color management
   - Expanded RGB colorspace support.
   - Added parameterized RGB/XYZ conversion helpers.
   - Added Bradford/CAT02 adaptation helpers.
   - Added ACES-related helper approximations.
   - Runtime scanning stage still needs integration to fully respect `io.output_color_space`.

6. Black/white and print balance services
   - Added profile/reference-aware state APIs in `ColorReferenceService` and `EnlargerService`.
   - Stage integration still needed; current stages still use older/simple calls.

7. Built-in profiles and neutral filter database
   - Added embedded `load_profile(stock)` support.
   - Added `available_profiles()`.
   - Added `init_params` helpers.
   - Added neutral print filter DB parsing/application in `digest_params`.

8. Params and Android update API
   - Added serde support and params JSON helpers.
   - Added patch/soft-update structs.
   - Added JNI/Kotlin update APIs.
   - Existing create/process/release APIs preserved.

9. Small model modules
   - Added `model/parametric.rs`.
   - Added `model/stocks.rs`.
   - Exposed both in `model/mod.rs`.

10. Android/headless buffer helpers
   - Added `utils/image_buffers.rs`.
   - Added u8/u16/f32/f64 buffer conversion and quantization helpers.
   - Added f32 preview resize helpers.

## Current diagnostics

Project diagnostics report no errors or warnings.

## Remaining implementation-only work

These are not testing tasks, but still need implementation/integration:

1. Run real Rust build once toolchain exists, then fix any compiler errors.
2. Wire `SpectralLUTService` into `SimulationPipeline`, `FilmingStage`, `PrintingStage`, and `ScanningStage`.
3. Wire new `ColorReferenceService`/`EnlargerService` print-balance methods into stages.
4. Update scanning to use new parameterized output color-space conversion functions.
5. Finish exact Mallett 2019 basis table embedding if exact parity is required.
6. Replace remaining approximations in diffusion/halation convolution if exact parity is required.
7. Add full Android asset/profile loading strategy if app should load profiles by name instead of JSON strings.
8. Decide whether desktop-only Python utilities are in scope for Android:
   - plotting
   - calibration target builder
   - OpenImageIO/exiv2/rawpy metadata and file I/O
   - raw file processor
   - FFT Gaussian implementation
9. Implement Vulkan/GPU spatial kernels if the README performance target is mandatory.
