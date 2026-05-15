# spectralfilm-rs Porting Audit

This audit compares the Python `src/spektrafilm` implementation with the current Rust/Android port in `spectralfilm-rs`.

## Summary

The current Rust port contains a complete end-to-end Android-native processing scaffold:

- profile JSON parsing
- runtime params
- filming stage
- printing stage
- scanning stage
- grain
- DIR couplers
- halation
- diffusion-filter placeholder model
- glare
- crop/resize
- JNI/C ABI
- Kotlin wrapper

However, it is not yet numerically equivalent to the Python implementation. Several Python features are simplified, missing, or only represented as scaffolding.

## Critical missing or incomplete items

### 1. Build validation has not been run

The environment used for this port did not have `cargo`, `rustc`, or `rustup`, so the Rust workspace has not been compiled.

Required first step after installing Rust:

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test --workspace`
- Android `cargo ndk ... build -p spectralfilm-ndk --release`

### 2. Exact Hanatos 2025 spectral LUT is not ported

Python uses:

- `src/spektrafilm/data/luts/spectral_upsampling/irradiance_xy_tc.npy`
- `compute_hanatos2025_tc_lut`
- cubic 2D LUT interpolation

Rust currently has a smooth spectral reconstruction fallback in `utils/spectral_upsampling.rs`. It preserves functionality but not numerical parity.

Needed:

- convert `.npy` LUT to a Rust-readable asset format or implement `.npy` reader
- implement the original triangular coordinate LUT path
- implement cubic 2D interpolation equivalent to Python `apply_lut_cubic_2d`
- cache film sensitivity TC LUT per profile/sensitivity

### 3. Enlarger/scanner spectral 3D LUT cache is not fully wired

Python has `SpectralLUTService` with:

- enlarger 3D LUT cache
- scanner 3D LUT cache
- cache invalidation via test samples
- optional `use_enlarger_lut`
- optional `use_scanner_lut`

Rust has a generic `Lut3d`, but pipeline stages do not use it yet.

Needed:

- port `runtime/services/spectral_lut_compute.py`
- wire `settings.use_enlarger_lut`
- wire `settings.use_scanner_lut`
- match Python LUT domains and interpolation behaviour

### 4. Diffusion filter model is simplified

Python `model/diffusion.py` has a detailed energy-conserving PSF model:

- `_DIFFUSION_FILTER_SHAPES`
- strength-to-scatter mapping
- core/halo/bloom groups
- per-family presets
- halo warmth redistribution
- analytic radial profile
- per-channel PSF
- FFT-based convolution for large kernels

Rust currently implements a simplified core/halo/bloom Gaussian blend.

Needed:

- port the full family config table
- port `_strength_to_scatter`
- port `_expand_group`
- port `_halo_channel_weights`
- port `_radial_components`
- port `diffusion_filter_radial_profile`
- port `diffusion_filter_psf`
- implement large-kernel convolution strategy

### 5. Halation scatter tail is simplified

Python uses a physically motivated scatter tail, including exponential-tail approximation via Gaussian mixture.

Rust currently models scatter with two Gaussian blurs.

Needed:

- port `fast_exponential_filter`
- use the same Gaussian-mixture coefficients as Python
- match `scatter_tail_um` behaviour exactly
- match boost ordering and renormalization options exactly

### 6. Color management is incomplete

Python uses `colour-science` for:

- RGB colourspace conversions
- chromatic adaptation
- output colourspace selection
- CCTF decoding/encoding
- illuminant xy handling
- ACES IDT helpers

Rust currently supports hard-coded sRGB/ProPhoto/Rec2020-ish paths and always outputs through an sRGB matrix in scanning.

Needed:

- support all Python colourspace names used by profiles/UI
- implement chromatic adaptation (CAT02/Bradford) consistently
- respect `io.output_color_space`
- respect `io.input_color_space`
- implement ACES conversion helpers or explicitly remove/replace if Android does not need them
- port ICC embedding only if image file I/O is kept native-side

### 7. Black/white reference correction is simplified

Python `ColorReferenceService` computes reference black/white densities and applies exposure corrections across filming/printing/scanning.

Rust currently does a conservative luminance remap in scanning and identity exposure corrections.

Needed:

- port `_update_cmy_black_white_references`
- port `black_white_filming_exposure_correction`
- port `black_white_printing_exposure_correction`
- port `black_white_xyz_correction`
- share cmy-to-log-xyz callable or equivalent Rust closure/service state

### 8. Print exposure normalization is incomplete

Python computes `density_spectral_midgray` and `density_spectral_midgray_comp` in `FilmingStage` and uses them in printing exposure factor logic.

Rust does not yet reproduce the full midgray balancing path.

Needed:

- port `_compute_density_spectral_midgray_to_balance_print`
- port `_simple_rgb_to_density_spectral`
- port `_compute_exposure_factor_midgray`
- implement `print_exposure_compensation` and `normalize_print_exposure` exactly

### 9. Neutral print filter database is copied but not applied

Python applies `data/filters/neutral_print_filters.json` during `digest_params`.

Rust has the JSON asset copied but does not parse/use it.

Needed:

- embed or load neutral filter database
- implement `apply_database_neutral_print_filters`
- preserve warning/missing behaviour or expose Android log warnings

### 10. Parameter updates and Android API are incomplete

Python supports:

- `Simulator.update_params`
- `Simulator.soft_update`
- many tweakable runtime params
- debug mode injection/output switches

Rust currently supports creation from film/print JSON and processing. It does not expose param JSON update or soft-update through JNI.

Needed:

- make runtime params serializable/deserializable
- add Android JSON parameter API
- expose soft-update fields through JNI/Kotlin
- implement inject debug mode pathway
- expose preview mode and `simulate_preview`

### 11. Stock/profile loading by name is missing in Rust API

Python has `load_profile(stock)` and `init_params(film_profile, print_profile)`.

Rust currently expects full profile JSON strings from Android.

Needed:

- embed built-in profiles with `include_str!` or package asset loader
- implement `load_profile(stock)` equivalent
- implement `init_params(film_profile, print_profile)` equivalent
- expose stock enums or constants

### 12. Exact filter data loading is missing

Python loads actual CSV filters:

- dichroic filters from multiple brands
- Schott KG1/KG3/KG5 data
- generic lens transmission

Rust currently uses analytic/custom filter approximations and a hard-coded KG3 approximation.

Needed:

- parse copied CSV assets
- implement filter interpolation equivalent to Python Akima interpolation or acceptable replacement
- support filter brands: `thorlabs`, `edmund_optics`, `durst_digital_light`, `custom`
- support heat absorbing/lens transmission filters

### 13. Illuminants are approximate

Python uses `colour` standard illuminants/light sources:

- D-series
- Incandescent
- Kinoton 75P
- blackbody
- TH-KG3
- TH-KG3-L

Rust contains tabulated/approximate values for common cases only.

Needed:

- verify D50/D55/D65 table values
- port exact `colour`-aligned data for all illuminants used by profiles
- apply KG3/lens transmission from actual asset CSVs

### 14. Gaussian/IIR filter parity must be verified

Python `fast_gaussian_filter.py` uses:

- reflect boundary mode
- fused FIR for small sigma
- Young & van Vliet IIR for large sigma
- exponential filter approximation

Rust has FIR/IIR code, but parity has not been tested and boundary handling differs in places.

Needed:

- write numeric baseline tests versus Python/scipy outputs
- correct IIR coefficients/boundaries if needed
- port exponential filter
- support per-channel sigma arrays

### 15. Grain stochastic parity is not guaranteed

Rust has grain simulation but uses Rust RNG distributions rather than scipy/Numba behaviour.

Needed:

- decide whether visual parity is enough or exact statistical parity is required
- match seed strategy more closely
- add RMS/mean/skewness tests from Python grain model
- expose deterministic seed controls if needed

### 16. Raw/image file I/O is not ported

Python has extensive desktop I/O:

- OpenImageIO image load/save
- EXIF/IPTC/XMP via exiv2
- ICC profile embedding
- rawpy RAW loading
- lensfun/exiv2 helpers

Rust Android port currently only processes buffers.

For Android this may be intentional, because Camera2/CameraX should provide buffers. If full Python functionality is required, native Android equivalents are needed.

Needed if keeping functionality:

- DNG/RAW ingestion strategy
- EXIF metadata copy/write strategy
- ICC profile output strategy
- Android Bitmap/ImageProxy conversion helpers

### 17. Desktop-only utilities are not ported

Not yet ported:

- `utils/calibration_targets.py`
- `utils/measure.py`
- `utils/plotting.py`
- `utils/raw_file_processor.py`
- `utils/numba_warmup.py`
- `utils/fft_gaussian_filter.py`
- `model/parametric.py`
- `model/stocks.py`

Some are not necessary for an Android headless engine, but they are part of the Python project.

Needed if preserving everything:

- port `parametric_density_curves_model`
- add stock enums/constants
- replace plotting/calibration UI utilities with Android/test equivalents or mark them explicitly out-of-scope

### 18. Tests/baselines are missing

The README mentions Rust regression snapshots, but no Rust tests/baselines have been created yet.

Needed:

- unit tests for profile parsing
- unit tests for density interpolation
- unit tests for spectral density composition
- unit tests for color filters/illuminants
- Python-vs-Rust golden output tests for small images
- Android JNI smoke test

### 19. Vulkan/GPU path is not implemented

README mentions moving heavy spatial operations to Vulkan compute.

Rust currently uses CPU convolution.

Needed for final Android performance target:

- Vulkan compute abstraction
- Gaussian/halation/diffusion kernels
- buffer interop strategy
- fallback CPU path

## Likely compile/build review items

These must be confirmed with `cargo check`:

- JNI signatures and lifetimes
- unused fields/imports as warnings
- exact Android logger feature compatibility
- all profile JSON shapes after flattening
- `panic = abort` behaviour with JNI exceptions
- FFI safety warnings for opaque `Engine`

## Recommended next implementation order

1. Install Rust/NDK and run `cargo check --workspace`.
2. Fix all compiler errors/warnings.
3. Port exact Hanatos spectral LUT path.
4. Port neutral print filter database use.
5. Port midgray print exposure normalization.
6. Port full diffusion filter PSF model.
7. Port exact ColorReferenceService corrections.
8. Add param JSON/update API to JNI/Kotlin.
9. Add built-in profile loading by stock name.
10. Add Python-vs-Rust regression tests.
11. Optimize performance and add Vulkan spatial kernels.

## Bottom line

The current Rust port is a strong Android-native foundation and contains an end-to-end simulation pipeline. What is still missing is mostly numerical parity and full API parity with the Python implementation, especially around LUTs, diffusion-filter PSFs, color management, print balancing, neutral filter database application, and test baselines.
