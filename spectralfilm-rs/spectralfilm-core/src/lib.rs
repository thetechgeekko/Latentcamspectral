//! spectralfilm-core: Physically-based spectral film simulation.
//!
//! Mirrors the Python `spektrafilm` package, ported to Rust for Android/NDK.
//! All heavy computation is done here; the `spectralfilm-ndk` crate exposes
//! a JNI C-ABI on top of this library.

pub mod config;
pub mod model;
pub mod profiles;
pub mod runtime;
pub mod utils;

pub use profiles::{Profile, ProfileData, ProfileInfo};
pub use runtime::{
    params::{RuntimePhotoParams, SimulationSettings},
    pipeline::SimulationPipeline,
};
