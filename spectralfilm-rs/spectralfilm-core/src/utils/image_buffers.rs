//! Android/headless-friendly image buffer helpers.
//!
//! The desktop Python implementation uses OpenImageIO, Exiv2, RawPy and package
//! resource file access for image decoding, metadata, RAW handling and profile
//! lookup. Those features are deliberately **not** mirrored here: on Android they
//! should live in the app layer, where platform codecs, content resolvers,
//! camera/RAW APIs and metadata libraries can be selected by the host
//! application. This module only handles already-decoded, tightly interleaved RGB
//! sample buffers and produces plain Rust `Vec` values suitable for the core
//! processing pipeline or for app-side encoders.
//!
//! All helpers are independent of JNI and Android framework types.

use core::fmt;

/// Number of channels expected by the helpers in this module.
pub const RGB_CHANNELS: usize = 3;

/// Public note for embedders: file, metadata, RAW and ICC I/O are app-side.
pub const ANDROID_APP_SIDE_IO_NOTE: &str = "spectralfilm-core intentionally does not perform image codec, filesystem, EXIF/XMP/IPTC, RAW, or ICC-profile I/O. Decode/encode and metadata handling should be provided by the Android host app, then pass tightly interleaved RGB buffers into these helpers.";

/// Errors returned when a supplied image buffer cannot represent the requested RGB image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferError {
    /// `width * height * 3` overflowed `usize`.
    DimensionOverflow { width: usize, height: usize },
    /// The caller supplied fewer samples than a tightly interleaved RGB image requires.
    BufferTooShort { required: usize, actual: usize },
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufferError::DimensionOverflow { width, height } => {
                write!(f, "RGB image dimensions overflow usize: {width}x{height}x3")
            }
            BufferError::BufferTooShort { required, actual } => {
                write!(f, "RGB buffer too short: required {required} samples, got {actual}")
            }
        }
    }
}

impl std::error::Error for BufferError {}

/// Return the number of scalar samples needed for a tightly interleaved RGB image.
pub fn rgb_sample_count(width: usize, height: usize) -> Result<usize, BufferError> {
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(RGB_CHANNELS))
        .ok_or(BufferError::DimensionOverflow { width, height })
}

fn validate_rgb_buffer<T>(buffer: &[T], width: usize, height: usize) -> Result<usize, BufferError> {
    let required = rgb_sample_count(width, height)?;
    if buffer.len() < required {
        return Err(BufferError::BufferTooShort { required, actual: buffer.len() });
    }
    Ok(required)
}

/// Clamp a floating-point sample into `[0, 1]`, mapping NaN/inf to `0`.
pub fn clamp_unit_f64(value: f64) -> f64 {
    if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 }
}

/// Clamp a floating-point sample into `[0, 1]`, mapping NaN/inf to `0`.
pub fn clamp_unit_f32(value: f32) -> f32 {
    if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 }
}

/// Quantize a normalized sample to `u8` after safe clamping.
pub fn quantize_unit_to_u8_f64(value: f64) -> u8 {
    (clamp_unit_f64(value) * u8::MAX as f64).round() as u8
}

/// Quantize a normalized sample to `u8` after safe clamping.
pub fn quantize_unit_to_u8_f32(value: f32) -> u8 {
    (clamp_unit_f32(value) * u8::MAX as f32).round() as u8
}

/// Quantize a normalized sample to `u16` after safe clamping.
pub fn quantize_unit_to_u16_f64(value: f64) -> u16 {
    (clamp_unit_f64(value) * u16::MAX as f64).round() as u16
}

/// Quantize a normalized sample to `u16` after safe clamping.
pub fn quantize_unit_to_u16_f32(value: f32) -> u16 {
    (clamp_unit_f32(value) * u16::MAX as f32).round() as u16
}

/// Convert tightly interleaved `u8` RGB samples to normalized `f64` samples.
pub fn rgb_u8_to_f64_normalized(buffer: &[u8], width: usize, height: usize) -> Result<Vec<f64>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| sample as f64 / u8::MAX as f64).collect())
}

/// Convert tightly interleaved `u8` RGB samples to normalized `f32` samples.
pub fn rgb_u8_to_f32_normalized(buffer: &[u8], width: usize, height: usize) -> Result<Vec<f32>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| sample as f32 / u8::MAX as f32).collect())
}

/// Convert tightly interleaved `u16` RGB samples to normalized `f64` samples.
pub fn rgb_u16_to_f64_normalized(buffer: &[u16], width: usize, height: usize) -> Result<Vec<f64>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| sample as f64 / u16::MAX as f64).collect())
}

/// Convert tightly interleaved `u16` RGB samples to normalized `f32` samples.
pub fn rgb_u16_to_f32_normalized(buffer: &[u16], width: usize, height: usize) -> Result<Vec<f32>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| sample as f32 / u16::MAX as f32).collect())
}

/// Normalize tightly interleaved `f32` RGB samples into `f64` samples.
///
/// Floating-point input is treated as already normalized image data; values
/// outside `[0, 1]` are clamped and NaN/inf are mapped to `0` so app-provided
/// buffers are safe to feed to integer encoders or preview code.
pub fn rgb_f32_to_f64_normalized(buffer: &[f32], width: usize, height: usize) -> Result<Vec<f64>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| clamp_unit_f32(sample) as f64).collect())
}

/// Normalize tightly interleaved `f32` RGB samples into `f32` samples.
pub fn rgb_f32_to_f32_normalized(buffer: &[f32], width: usize, height: usize) -> Result<Vec<f32>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| clamp_unit_f32(sample)).collect())
}

/// Clamp normalized `f64` RGB samples and return a `f32` buffer.
pub fn f64_rgb_to_f32_normalized(buffer: &[f64], width: usize, height: usize) -> Result<Vec<f32>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| clamp_unit_f64(sample) as f32).collect())
}

/// Clamp normalized `f32` RGB samples and return a `f32` buffer.
pub fn f32_rgb_to_f32_normalized(buffer: &[f32], width: usize, height: usize) -> Result<Vec<f32>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| clamp_unit_f32(sample)).collect())
}

/// Clamp and quantize normalized `f64` RGB samples into interleaved `u8` output.
pub fn f64_rgb_to_u8(buffer: &[f64], width: usize, height: usize) -> Result<Vec<u8>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| quantize_unit_to_u8_f64(sample)).collect())
}

/// Clamp and quantize normalized `f32` RGB samples into interleaved `u8` output.
pub fn f32_rgb_to_u8(buffer: &[f32], width: usize, height: usize) -> Result<Vec<u8>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| quantize_unit_to_u8_f32(sample)).collect())
}

/// Clamp and quantize normalized `f64` RGB samples into interleaved `u16` output.
pub fn f64_rgb_to_u16(buffer: &[f64], width: usize, height: usize) -> Result<Vec<u16>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| quantize_unit_to_u16_f64(sample)).collect())
}

/// Clamp and quantize normalized `f32` RGB samples into interleaved `u16` output.
pub fn f32_rgb_to_u16(buffer: &[f32], width: usize, height: usize) -> Result<Vec<u16>, BufferError> {
    let required = validate_rgb_buffer(buffer, width, height)?;
    Ok(buffer[..required].iter().map(|&sample| quantize_unit_to_u16_f32(sample)).collect())
}
