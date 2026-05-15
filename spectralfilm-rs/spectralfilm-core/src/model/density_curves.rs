//! H&D density-curve interpolation.
//!
//! Mirrors `spektrafilm/model/density_curves.py`.

use crate::utils::interp::linear_interp_clamped;

/// Interpolate log-exposure values into CMY density using the H&D curves.
///
/// # Arguments
/// * `log_raw` – image log-exposure, shape [H × W × 3] (row-major flat).
/// * `log_exposure` – exposure axis, length N_LE.
/// * `density_curves` – H&D curves, shape [N_LE × 3] (row-major flat).
/// * `gamma_factor` – per-channel gamma multiplier [3]; scales the exposure axis.
///
/// # Returns
/// Flat `Vec<f64>` of shape [H × W × 3].
pub fn interpolate_exposure_to_density(
    log_raw: &[f64],
    log_exposure: &[f64],
    density_curves: &[f64],
    gamma_factor: [f64; 3],
) -> Vec<f64> {
    let n_pixels = log_raw.len() / 3;
    let n_le = log_exposure.len();
    let mut out = vec![0.0f64; log_raw.len()];

    // Per channel: scale the exposure axis by gamma, then interpolate.
    for ch in 0..3 {
        let gf = gamma_factor[ch];
        // Build scaled exposure axis (avoid heap alloc by computing on the fly).
        let x_at = |k: usize| log_exposure[k] / gf;
        let y_at = |k: usize| density_curves[k * 3 + ch];

        for px in 0..n_pixels {
            let x = log_raw[px * 3 + ch];
            out[px * 3 + ch] = interp1d_clamped(x, n_le, x_at, y_at);
        }
    }
    out
}

/// Same as above but operates in-place and supports rayon parallelism.
pub fn interpolate_exposure_to_density_par(
    log_raw: &[f64],
    log_exposure: &[f64],
    density_curves: &[f64],
    gamma_factor: [f64; 3],
    out: &mut [f64],
) {
    use rayon::prelude::*;
    let n_le = log_exposure.len();

    // Process each channel independently (no data races: channels stride by 3).
    for ch in 0..3 {
        let gf = gamma_factor[ch];
        let slice_in: Vec<f64> = log_raw.iter().skip(ch).step_by(3).copied().collect();
        let slice_out: Vec<f64> = slice_in
            .par_iter()
            .map(|&x| {
                interp1d_clamped(x, n_le, |k| log_exposure[k] / gf, |k| density_curves[k * 3 + ch])
            })
            .collect();
        for (i, &v) in slice_out.iter().enumerate() {
            out[i * 3 + ch] = v;
        }
    }
}

/// 1-D linear interpolation with clamped extrapolation.
///
/// `x_at(k)` returns the k-th point on the x axis (can be a closure).
/// `y_at(k)` returns the k-th point on the y axis.
#[inline]
pub fn interp1d_clamped<FX, FY>(x: f64, n: usize, x_at: FX, y_at: FY) -> f64
where
    FX: Fn(usize) -> f64,
    FY: Fn(usize) -> f64,
{
    if x <= x_at(0) {
        return y_at(0);
    }
    if x >= x_at(n - 1) {
        return y_at(n - 1);
    }
    // Binary search for the bracketing interval.
    let mut lo = 0usize;
    let mut hi = n - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if x_at(mid) <= x { lo = mid; } else { hi = mid; }
    }
    let x0 = x_at(lo);
    let x1 = x_at(hi);
    let dx = x1 - x0;
    if dx.abs() < 1e-30 {
        return y_at(lo);
    }
    let t = (x - x0) / dx;
    y_at(lo) + t * (y_at(hi) - y_at(lo))
}

/// Interpolate CMY densities into per-sublayer densities.
///
/// Returns flat `Vec<f64>` of shape [H × W × 3 × 3] (last two: [layer, ch]).
pub fn interp_density_cmy_layers(
    density_cmy: &[f64],
    density_curves: &[f64],     // N_LE × 3
    density_curves_layers: &[f64], // N_LE × 3 × 3
    positive_film: bool,
    n_le: usize,
) -> Vec<f64> {
    let n_pixels = density_cmy.len() / 3;
    // Output: [px, ch, layer] or equivalently [px * 9 + ch * 3 + layer]
    // We'll use [px * 9 + layer * 3 + ch] to match Python convention [x,y,layer,rgb]
    let mut out = vec![0.0f64; n_pixels * 9];

    for ch in 0..3usize {
        for layer in 0..3usize {
            for px in 0..n_pixels {
                let d = density_cmy[px * 3 + ch];
                // For positive film: negate both axes (density decreases with exposure)
                let val = if positive_film {
                    let neg_d = -d;
                    let x_at = |k: usize| -density_curves[k * 3 + ch];
                    let y_at = |k: usize| density_curves_layers[k * 9 + layer * 3 + ch];
                    interp1d_clamped(neg_d, n_le, x_at, y_at)
                } else {
                    let x_at = |k: usize| density_curves[k * 3 + ch];
                    let y_at = |k: usize| density_curves_layers[k * 9 + layer * 3 + ch];
                    interp1d_clamped(d, n_le, x_at, y_at)
                };
                out[px * 9 + layer * 3 + ch] = val;
            }
        }
    }
    out
}
