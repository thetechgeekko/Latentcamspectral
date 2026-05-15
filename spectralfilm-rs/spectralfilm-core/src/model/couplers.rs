//! DIR (developer inhibitor-releasing) coupler model.
//!
//! Mirrors `spektrafilm/model/couplers.py`.

use crate::model::density_curves::interpolate_exposure_to_density;
use crate::utils::gaussian::gaussian_filter_2d;

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DirCouplersParams {
    pub active: bool,
    pub amount: f64,
    pub inhibition_samelayer: f64,
    pub inhibition_interlayer: f64,
    /// Same-layer gamma for R, G, B channels.
    pub gamma_samelayer_rgb: [f64; 3],
    /// Interlayer R→G, R→B.
    pub gamma_interlayer_r_to_gb: [f64; 2],
    /// Interlayer G→R, G→B.
    pub gamma_interlayer_g_to_rb: [f64; 2],
    /// Interlayer B→R, B→G.
    pub gamma_interlayer_b_to_rg: [f64; 2],
    /// Diffusion sigma (µm) of the inhibitor cloud.
    pub diffusion_size_um: f64,
}

impl Default for DirCouplersParams {
    fn default() -> Self {
        Self {
            active: true,
            amount: 1.0,
            inhibition_samelayer: 1.0,
            inhibition_interlayer: 1.0,
            gamma_samelayer_rgb: [0.341, 0.324, 0.273],
            gamma_interlayer_r_to_gb: [0.355, 0.305],
            gamma_interlayer_g_to_rb: [0.154, 0.358],
            gamma_interlayer_b_to_rg: [0.171, 0.225],
            diffusion_size_um: 20.0,
        }
    }
}

// ---------------------------------------------------------------------------
// 3×3 coupling matrix
// ---------------------------------------------------------------------------

/// Build the 3×3 DIR coupler matrix M where M[donor, receiver] = gamma.
/// Row = donor layer, column = affected layer.
pub fn compute_dir_couplers_matrix(p: &DirCouplersParams) -> [[f64; 3]; 3] {
    let mut m = [[0.0f64; 3]; 3];
    // Diagonal (same-layer)
    m[0][0] = p.gamma_samelayer_rgb[0] * p.inhibition_samelayer;
    m[1][1] = p.gamma_samelayer_rgb[1] * p.inhibition_samelayer;
    m[2][2] = p.gamma_samelayer_rgb[2] * p.inhibition_samelayer;
    // Off-diagonal (interlayer)
    m[0][1] = p.gamma_interlayer_r_to_gb[0] * p.inhibition_interlayer;
    m[0][2] = p.gamma_interlayer_r_to_gb[1] * p.inhibition_interlayer;
    m[1][0] = p.gamma_interlayer_g_to_rb[0] * p.inhibition_interlayer;
    m[1][2] = p.gamma_interlayer_g_to_rb[1] * p.inhibition_interlayer;
    m[2][0] = p.gamma_interlayer_b_to_rg[0] * p.inhibition_interlayer;
    m[2][1] = p.gamma_interlayer_b_to_rg[1] * p.inhibition_interlayer;
    m
}

// ---------------------------------------------------------------------------
// Density curves before DIR couplers (invert coupler effect from published curves)
// ---------------------------------------------------------------------------

pub fn compute_density_curves_before_dir_couplers(
    density_curves: &[f64],  // N_LE × 3
    log_exposure: &[f64],    // N_LE
    couplers_matrix: &[[f64; 3]; 3],
    positive: bool,
    n_le: usize,
) -> Vec<f64> {
    let mut dc_silver = density_curves.to_vec();
    if positive {
        // silver density = d_max - d
        let mut dmax = [0.0f64; 3];
        for le in 0..n_le {
            for ch in 0..3 {
                let v = density_curves[le * 3 + ch];
                if v.is_finite() && v > dmax[ch] { dmax[ch] = v; }
            }
        }
        for le in 0..n_le {
            for ch in 0..3 {
                dc_silver[le * 3 + ch] = dmax[ch] - density_curves[le * 3 + ch];
            }
        }
    }

    // coupler_amount[le, receiver] = sum_donor( dc_silver[le,donor] * M[donor,receiver] )
    let mut coupler_amount = vec![0.0f64; n_le * 3];
    for le in 0..n_le {
        for receiver in 0..3 {
            let mut sum = 0.0;
            for donor in 0..3 {
                sum += dc_silver[le * 3 + donor] * couplers_matrix[donor][receiver];
            }
            coupler_amount[le * 3 + receiver] = sum;
        }
    }

    // Shifted log-exposure axis: le_0[le, ch] = log_exposure[le] - coupler_amount[le,ch]
    let mut dc_corrected = vec![0.0f64; n_le * 3];
    for ch in 0..3 {
        // Build shifted x and original y, then re-interpolate
        let le_shifted: Vec<f64> = (0..n_le).map(|k| log_exposure[k] - coupler_amount[k * 3 + ch]).collect();
        let y_orig: Vec<f64> = (0..n_le).map(|k| density_curves[k * 3 + ch]).collect();
        for le in 0..n_le {
            let x = log_exposure[le];
            let v = if positive {
                -interp1d_sorted_clamped(x, &le_shifted, &y_orig.iter().map(|&v| -v).collect::<Vec<_>>())
            } else {
                interp1d_sorted_clamped(x, &le_shifted, &y_orig)
            };
            dc_corrected[le * 3 + ch] = v;
        }
    }
    dc_corrected
}

fn interp1d_sorted_clamped(x: f64, xs: &[f64], ys: &[f64]) -> f64 {
    use crate::model::density_curves::interp1d_clamped;
    let n = xs.len();
    interp1d_clamped(x, n, |k| xs[k], |k| ys[k])
}

// ---------------------------------------------------------------------------
// Exposure correction from dir couplers
// ---------------------------------------------------------------------------

pub fn compute_exposure_correction_dir_couplers(
    log_raw: &[f64],          // N_PX × 3
    density_cmy: &[f64],      // N_PX × 3
    density_max: &[f64; 3],
    couplers_matrix: &[[f64; 3]; 3],
    diffusion_size_px: f64,
    positive: bool,
    width: usize,
    height: usize,
) -> Vec<f64> {
    let n_px = width * height;

    // Silver density
    let mut density_silver: Vec<f64> = density_cmy.to_vec();
    if positive {
        for px in 0..n_px {
            for ch in 0..3 {
                density_silver[px * 3 + ch] = density_max[ch] - density_cmy[px * 3 + ch];
            }
        }
    }

    // log_raw_correction[px, receiver] = sum_donor( density_silver[px,donor] * M[donor,receiver] )
    let mut correction = vec![0.0f64; n_px * 3];
    for px in 0..n_px {
        for receiver in 0..3 {
            let mut sum = 0.0;
            for donor in 0..3 {
                sum += density_silver[px * 3 + donor] * couplers_matrix[donor][receiver];
            }
            correction[px * 3 + receiver] = sum;
        }
    }

    // Spatial diffusion
    if diffusion_size_px > 0.0 {
        for ch in 0..3 {
            let mut slice: Vec<f64> = (0..n_px).map(|px| correction[px * 3 + ch]).collect();
            slice = gaussian_filter_2d(&slice, width, height, diffusion_size_px);
            for px in 0..n_px { correction[px * 3 + ch] = slice[px]; }
        }
    }

    // Apply correction
    let mut out = vec![0.0f64; n_px * 3];
    for px in 0..n_px {
        for ch in 0..3 {
            out[px * 3 + ch] = log_raw[px * 3 + ch] - correction[px * 3 + ch];
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Public apply function
// ---------------------------------------------------------------------------

pub fn apply_density_correction_dir_couplers(
    density_cmy: &[f64],
    log_raw: &[f64],
    pixel_size_um: f64,
    log_exposure: &[f64],
    density_curves: &[f64],
    p: &DirCouplersParams,
    profile_type: &str,
    gamma_factor: f64,
    shape: (usize, usize),
) -> Vec<f64> {
    if !p.active {
        return density_cmy.to_vec();
    }
    let positive = profile_type == "positive";
    let (h, w) = shape;
    let n_le = log_exposure.len();

    let mut m = compute_dir_couplers_matrix(p);
    for row in &mut m { for v in row.iter_mut() { *v *= p.amount; } }

    let dc_before = compute_density_curves_before_dir_couplers(
        density_curves, log_exposure, &m, positive, n_le,
    );

    let mut dmax = [0.0f64; 3];
    for le in 0..n_le {
        for ch in 0..3 {
            let v = density_curves[le * 3 + ch];
            if v.is_finite() && v > dmax[ch] { dmax[ch] = v; }
        }
    }

    let diffusion_px = p.diffusion_size_um / pixel_size_um;
    let log_raw_0 = compute_exposure_correction_dir_couplers(
        log_raw, density_cmy, &dmax, &m, diffusion_px, positive, w, h,
    );

    interpolate_exposure_to_density(&log_raw_0, log_exposure, &dc_before, [gamma_factor; 3])
}
