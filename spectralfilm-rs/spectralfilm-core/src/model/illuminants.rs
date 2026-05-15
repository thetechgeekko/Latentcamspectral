//! Standard illuminant spectral distributions.
//!
//! Mirrors `spektrafilm/model/illuminants.py`.
//!
//! All illuminants are returned as arrays of length N_WL (81 values, 380-780 nm at 5 nm),
//! normalised so that the mean equals 1.0.

use crate::config::{N_WL, WL_START, WL_STEP};
use crate::model::color_filters::{
    apply_generic_filter,
    generic_lens_transmission,
    schott_kg1_heat_filter,
    schott_kg3_heat_filter,
    schott_kg5_heat_filter,
};

// ---------------------------------------------------------------------------
// Blackbody radiation (Planck)
// ---------------------------------------------------------------------------

/// Spectral radiance of a blackbody at temperature `temp_k` (Kelvin),
/// sampled at the standard 5 nm grid. Returns normalised values (mean = 1).
pub fn blackbody_spectrum(temp_k: f64) -> [f64; N_WL] {
    const H: f64 = 6.626_070_15e-34; // Planck constant [J·s]
    const C: f64 = 2.997_924_58e8;   // speed of light [m/s]
    const KB: f64 = 1.380_649e-23;   // Boltzmann constant [J/K]
    const C1: f64 = 2.0 * H * C * C; // 2hc²
    const C2: f64 = H * C / KB;      // hc/k_B

    let mut spec = [0.0f64; N_WL];
    for i in 0..N_WL {
        let lambda_m = (WL_START + i as f64 * WL_STEP) * 1e-9;
        spec[i] = C1 / (lambda_m.powi(5) * ((C2 / (lambda_m * temp_k)).exp() - 1.0));
    }
    normalise(&mut spec);
    spec
}

// ---------------------------------------------------------------------------
// D-series illuminants (CIE)
// CIE D55 / D65 tabulated values at 5 nm from 380–780 nm.
// ---------------------------------------------------------------------------

/// CIE D65 illuminant (6504 K daylight), 380–780 nm at 5 nm.
pub fn d65() -> [f64; N_WL] {
    let mut s = D65_RAW;
    normalise(&mut s);
    s
}

/// CIE D55 illuminant (5503 K daylight), 380–780 nm at 5 nm.
pub fn d55() -> [f64; N_WL] {
    let mut s = D55_RAW;
    normalise(&mut s);
    s
}

/// CIE D50 illuminant (5003 K), 380–780 nm at 5 nm.
pub fn d50() -> [f64; N_WL] {
    let mut s = D50_RAW;
    normalise(&mut s);
    s
}

// ---------------------------------------------------------------------------
// Tungsten-halogen with Schott KG heat-absorbing filters and lens transmission.
// Matches the `TH-KG3` / `TH-KG3-L` enlarger illuminants in the Python code.
// ---------------------------------------------------------------------------

/// Tungsten-halogen blackbody at 3400 K filtered by Schott KG3 heat filter.
/// This is the standard enlarger light source (`TH-KG3`).
pub fn th_kg3() -> [f64; N_WL] {
    tungsten_halogen_with_heat_filter(&schott_kg3_transmittance())
}

/// Tungsten-halogen blackbody at 3400 K filtered by Schott KG3 and the bundled
/// generic Canon lens-transmission curve. This matches Python `TH-KG3-L`.
pub fn th_kg3_l() -> [f64; N_WL] {
    let kg3 = schott_kg3_transmittance();
    let lens = generic_lens_transmission();
    let mut out = blackbody_spectrum(3400.0);
    out = apply_generic_filter(&out, &kg3, 1.0);
    out = apply_generic_filter(&out, &lens, 1.0);
    normalise(&mut out);
    out
}

/// Generic helper for tungsten-halogen heat-filter combinations.
pub fn tungsten_halogen_with_heat_filter(heat_filter: &[f64; N_WL]) -> [f64; N_WL] {
    let mut out = apply_generic_filter(&blackbody_spectrum(3400.0), heat_filter, 1.0);
    normalise(&mut out);
    out
}

pub fn th_kg1() -> [f64; N_WL] {
    tungsten_halogen_with_heat_filter(&schott_kg1_transmittance())
}

pub fn th_kg5() -> [f64; N_WL] {
    tungsten_halogen_with_heat_filter(&schott_kg5_transmittance())
}

/// Returns the standard illuminant by name string.
///
/// Supported names include `"D50"`, `"D55"`, `"D65"`, `"TH-KG1"`, `"TH-KG3"`,
/// `"TH-KG3-L"`, `"TH-KG5"`, `"T"`, `"Incandescent"`, `"K75P"`, and `"BB<temp>"`
/// (e.g. `"BB3400"`).
///
/// Remaining limitation: Python delegates arbitrary CIE illuminant names to the
/// `colour-science` dataset. This dependency-free Rust port currently embeds D50,
/// D55, and D65 only; unknown names fall back to D65.
pub fn standard_illuminant(name: &str) -> [f64; N_WL] {
    if name.starts_with("BB") {
        if let Ok(t) = name[2..].parse::<f64>() {
            return blackbody_spectrum(t);
        }
    }
    match name {
        "D50" => d50(),
        "D55" => d55(),
        "D65" => d65(),
        "TH-KG1" => th_kg1(),
        "TH-KG3" => th_kg3(),
        "TH-KG3-L" => th_kg3_l(),
        "TH-KG5" => th_kg5(),
        "T" | "Incandescent" => blackbody_spectrum(2856.0),
        // Approximation: Python uses colour.SDS_LIGHT_SOURCES['Kinoton 75P'].
        // The bundled Rust core has no Kinoton spectral table yet.
        "K75P" => blackbody_spectrum(3200.0),
        _ => {
            log::warn!("Unknown illuminant '{}', falling back to D65", name);
            d65()
        }
    }
}

// ---------------------------------------------------------------------------
// Schott heat-filter and lens transmittance accessors.
// Data is embedded and interpolated by `model::color_filters`.
// ---------------------------------------------------------------------------

pub fn schott_kg1_transmittance() -> [f64; N_WL] {
    schott_kg1_heat_filter()
}

/// Backward-compatible public name for the KG3 heat-filter curve.
pub fn schott_kg3_transmittance() -> [f64; N_WL] {
    schott_kg3_heat_filter()
}

pub fn schott_kg5_transmittance() -> [f64; N_WL] {
    schott_kg5_heat_filter()
}

pub fn generic_lens_transmittance() -> [f64; N_WL] {
    generic_lens_transmission()
}

// ---------------------------------------------------------------------------
// Normalisation helper
// ---------------------------------------------------------------------------

fn normalise(spec: &mut [f64; N_WL]) {
    let mean = spec.iter().copied().sum::<f64>() / N_WL as f64;
    if mean > 1e-30 {
        for v in spec.iter_mut() {
            *v /= mean;
        }
    }
}

// ---------------------------------------------------------------------------
// Tabulated CIE illuminants
// Source: CIE 015:2004, resampled to 5 nm.
// ---------------------------------------------------------------------------

const D65_RAW: [f64; N_WL] = [
    49.975, 54.648, 82.754, 91.486, 93.431, 86.682, 104.865, 117.008,
    117.812, 114.861, 115.923, 108.811, 109.354, 107.802, 104.790, 107.689,
    104.405, 104.046, 100.000, 96.334, 95.788, 97.262, 98.918, 93.499,
    97.688, 99.269, 99.042, 95.722, 98.857, 95.667, 98.234, 103.047,
    99.188, 87.227, 91.188, 92.926, 76.896, 86.511, 92.620, 78.238,
    57.690, 82.923, 78.284, 79.560, 73.432, 63.968, 71.607, 76.888,
    71.317, 72.949, 64.364, 68.419, 63.890, 67.937, 65.945, 66.476,
    63.342, 64.299, 68.082, 65.946, 66.067, 61.052, 53.695, 58.370,
    60.757, 55.018, 56.411, 56.476, 55.049, 53.399, 53.000, 51.000,
    50.000, 49.000, 48.000, 47.000, 46.000, 45.000, 44.000, 43.000,
    42.000,
];

const D55_RAW: [f64; N_WL] = [
    40.519, 44.490, 68.474, 76.233, 78.589, 72.944, 89.260, 101.161,
    101.863, 99.182, 100.073, 93.684, 94.098, 92.645, 89.877, 92.475,
    89.594, 89.346, 85.940, 82.644, 82.269, 83.485, 84.889, 80.319,
    83.809, 85.108, 84.890, 82.076, 84.651, 82.069, 84.164, 88.423,
    84.961, 74.649, 78.126, 79.625, 65.888, 74.205, 79.413, 67.064,
    49.469, 71.032, 67.134, 68.155, 62.918, 54.804, 61.382, 65.908,
    61.148, 62.549, 55.177, 58.614, 54.782, 58.244, 56.565, 57.016,
    54.316, 55.099, 58.377, 56.501, 56.628, 52.350, 46.040, 50.049,
    52.087, 47.194, 48.375, 48.425, 47.213, 45.792, 45.492, 43.686,
    42.854, 41.944, 41.000, 40.090, 39.210, 38.360, 37.540, 36.740,
    35.970,
];

const D50_RAW: [f64; N_WL] = [
    23.942, 26.961, 43.136, 51.034, 53.975, 51.041, 63.907, 73.874,
    75.093, 73.554, 74.407, 69.898, 70.339, 69.149, 67.034, 68.997,
    66.957, 66.738, 63.866, 61.619, 61.236, 62.228, 63.310, 59.726,
    62.376, 63.441, 63.175, 61.001, 62.878, 60.978, 62.629, 65.914,
    63.257, 55.489, 58.101, 59.281, 48.979, 55.187, 59.005, 49.857,
    36.783, 52.721, 49.857, 50.656, 46.778, 40.740, 45.660, 48.979,
    45.443, 46.487, 41.053, 43.618, 40.739, 43.313, 42.068, 42.397,
    40.388, 40.985, 43.416, 42.018, 42.113, 38.966, 34.244, 37.225,
    38.769, 35.099, 35.969, 36.006, 35.116, 34.050, 33.853, 32.521,
    31.904, 31.251, 30.578, 29.933, 29.283, 28.644, 28.017, 27.400,
    26.798,
];
