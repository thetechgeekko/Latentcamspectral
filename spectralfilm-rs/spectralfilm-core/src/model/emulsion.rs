//! Emulsion develop helpers.
//!
//! Mirrors `spektrafilm/model/emulsion.py`.

use crate::config::N_WL;
use crate::model::{
    couplers::{apply_density_correction_dir_couplers, DirCouplersParams},
    density_curves::interpolate_exposure_to_density,
    grain::{apply_grain, GrainParams},
};

// ---------------------------------------------------------------------------
// Spectral density composition
// ---------------------------------------------------------------------------

/// Compose spectral density from CMY densities and per-channel dye absorption.
///
/// `density_cmy` : [N_PX × 3] flat slice
/// `channel_density` : [N_WL × 3] flat slice
/// `base_density` : [N_WL] slice
///
/// Returns `Vec<f64>` of shape [N_PX × N_WL].
pub fn compute_density_spectral(
    channel_density: &[f64],
    density_cmy: &[f64],
    base_density: &[f64],
    n_wl: usize,
) -> Vec<f64> {
    let n_px = density_cmy.len() / 3;
    let mut out = vec![0.0f64; n_px * n_wl];
    // density_spectral[px, wl] = sum_ch( density_cmy[px,ch] * channel_density[wl,ch] ) + base[wl]
    for px in 0..n_px {
        for wl in 0..n_wl {
            let mut val = base_density[wl];
            for ch in 0..3 {
                let d_ch = density_cmy[px * 3 + ch];
                let cd = channel_density[wl * 3 + ch];
                // NaN-safe: treat NaN channel_density as 0
                if cd.is_finite() && d_ch.is_finite() {
                    val += d_ch * cd;
                }
            }
            out[px * n_wl + wl] = val;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Simple develop (density curves only, no grain/couplers)
// ---------------------------------------------------------------------------

/// Map log-raw exposure through H&D density curves (no grain, no couplers).
pub fn develop_simple(
    log_raw: &[f64],
    log_exposure: &[f64],
    density_curves: &[f64],
    gamma_factor: [f64; 3],
) -> Vec<f64> {
    interpolate_exposure_to_density(log_raw, log_exposure, density_curves, gamma_factor)
}

// ---------------------------------------------------------------------------
// Full develop (H&D + DIR couplers + grain)
// ---------------------------------------------------------------------------

pub struct DevelopParams<'a> {
    pub log_exposure: &'a [f64],
    pub density_curves: &'a [f64],
    pub density_curves_layers: &'a [f64],
    pub dir_couplers: &'a DirCouplersParams,
    pub grain: &'a GrainParams,
    pub profile_type: &'a str, // "negative" | "positive"
    pub gamma_factor: f64,
    pub pixel_size_um: f64,
}

/// Full emulsion development: H&D curves → DIR couplers → grain.
pub fn develop(log_raw: &[f64], params: &DevelopParams, out_shape: (usize, usize)) -> Vec<f64> {
    let (h, w) = out_shape;
    let n_px = h * w;

    let dc = params.density_curves;
    let le = params.log_exposure;
    let n_le = le.len();

    // Normalised density curves (min subtracted per channel)
    let dc_norm = normalise_density_curves(dc, n_le);

    let gamma = [params.gamma_factor; 3];
    let mut density_cmy = interpolate_exposure_to_density(log_raw, le, &dc_norm, gamma);

    // DIR couplers
    density_cmy = apply_density_correction_dir_couplers(
        &density_cmy,
        log_raw,
        params.pixel_size_um,
        le,
        &dc_norm,
        params.dir_couplers,
        params.profile_type,
        params.gamma_factor,
        (h, w),
    );

    // Grain
    apply_grain(&density_cmy, params.pixel_size_um, params.grain, &dc_norm,
                params.density_curves_layers, n_le, params.profile_type, (h, w))
}

/// Subtract the per-channel minimum from density curves.
pub fn normalise_density_curves(density_curves: &[f64], n_le: usize) -> Vec<f64> {
    let mut out = density_curves.to_vec();
    for ch in 0..3 {
        let mut min_val = f64::INFINITY;
        for le in 0..n_le {
            let v = density_curves[le * 3 + ch];
            if v.is_finite() && v < min_val { min_val = v; }
        }
        if !min_val.is_finite() { min_val = 0.0; }
        for le in 0..n_le {
            out[le * 3 + ch] -= min_val;
        }
    }
    out
}
