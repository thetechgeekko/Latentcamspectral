//! Stochastic silver-halide grain simulation.
//!
//! Mirrors `spektrafilm/model/grain.py`.
//! Uses a Poisson-Binomial particle model per colour layer per pixel.

use rand::SeedableRng;
use rand::rngs::SmallRng;
use rayon::prelude::*;

use crate::model::density_curves::interp_density_cmy_layers;
use crate::utils::gaussian::gaussian_filter_2d;
use crate::utils::fast_stats::{fast_poisson_scalar, fast_binomial_scalar, fast_lognormal_from_mean_std_scalar};

// ---------------------------------------------------------------------------
// Parameter structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GrainParams {
    pub active: bool,
    pub sublayers_active: bool,
    /// Mean AgX grain area (µm²).
    pub agx_particle_area_um2: f64,
    /// Per-channel (R,G,B) grain-size scale multiplier.
    pub agx_particle_scale: [f64; 3],
    /// Per-sublayer grain-size scale multiplier.
    pub agx_particle_scale_layers: [f64; 3],
    /// Dmin per channel [density units].
    pub density_min: [f64; 3],
    /// Grain clumping uniformity per channel (0..1, high = uniform).
    pub uniformity: [f64; 3],
    /// Post-grain Gaussian blur sigma (pixels).
    pub blur: f64,
    /// Per-particle dye-cloud blur radius (µm).
    pub blur_dye_clouds_um: f64,
    /// Micro-structure (blur_frac, sigma_nm).
    pub micro_structure: (f64, f64),
    /// Number of sub-layers (only used when sublayers_active = false).
    pub n_sub_layers: usize,
    /// Deterministic RNG seed offset. If None, default [0, 1, 2] is used.
    pub fixed_seed: Option<u64>,
}

impl Default for GrainParams {
    fn default() -> Self {
        Self {
            active: true,
            sublayers_active: true,
            agx_particle_area_um2: 0.2,
            agx_particle_scale: [0.8, 1.0, 2.0],
            agx_particle_scale_layers: [2.5, 1.0, 0.5],
            density_min: [0.07, 0.08, 0.12],
            uniformity: [0.97, 0.97, 0.99],
            blur: 0.65,
            blur_dye_clouds_um: 1.0,
            micro_structure: (0.2, 30.0),
            n_sub_layers: 1,
            fixed_seed: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Core per-pixel particle model
// ---------------------------------------------------------------------------

/// Poisson-Binomial single-channel grain simulation for one pixel.
///
/// Returns the stochastic density value.
#[inline]
fn particle_model_pixel(
    density: f64,
    density_max: f64,
    n_particles: f64,
    uniformity: f64,
    rng: &mut SmallRng,
) -> f64 {
    let p = (density / density_max).clamp(1e-6, 1.0 - 1e-6);
    let od_particle = density_max / n_particles;
    let saturation = 1.0 - p * uniformity * (1.0 - 1e-6);

    let lambda = (n_particles / saturation).max(0.01);
    let seeds = fast_poisson_scalar(lambda, rng);
    
    let k = if seeds > 0 {
        fast_binomial_scalar(seeds, p, rng) as f64
    } else {
        0.0
    };
    k * od_particle * saturation
}

// ---------------------------------------------------------------------------
// Single-channel grain for a full image slice
// ---------------------------------------------------------------------------

fn apply_grain_channel(
    density_slice: &[f64], // [n_pixels]
    density_max: f64,
    n_particles: f64,
    uniformity: f64,
    seed: u64,
    blur_sigma: f64,
    width: usize,
    height: usize,
) -> Vec<f64> {
    let n_px = density_slice.len();
    let mut out = vec![0.0f64; n_px];
    // Parallel rows; each row gets its own RNG seeded deterministically.
    out.par_chunks_mut(width)
        .enumerate()
        .for_each(|(row, row_out)| {
            let row_seed = seed.wrapping_add(row as u64 * 2654435761);
            let mut rng = SmallRng::seed_from_u64(row_seed);
            for (col, v) in row_out.iter_mut().enumerate() {
                let px = row * width + col;
                let d = density_slice[px];
                *v = particle_model_pixel(d, density_max, n_particles, uniformity, &mut rng);
            }
        });

    if blur_sigma > 0.4 {
        out = gaussian_filter_2d(&out, width, height, blur_sigma);
    }
    out
}

// ---------------------------------------------------------------------------
// Simple grain (no sublayers)
// ---------------------------------------------------------------------------

fn apply_grain_simple(
    density_cmy: &[f64],
    width: usize,
    height: usize,
    pixel_size_um: f64,
    g: &GrainParams,
    density_curves_norm: &[f64],
    n_le: usize,
) -> Vec<f64> {
    let n_px = width * height;
    // Max density per channel (row max of density_curves)
    let mut density_max = [0.0f64; 3];
    for le in 0..n_le {
        for ch in 0..3 {
            let v = density_curves_norm[le * 3 + ch];
            if v.is_finite() && v > density_max[ch] { density_max[ch] = v; }
        }
    }

    let pixel_area = pixel_size_um * pixel_size_um;
    let mut out = vec![0.0f64; n_px * 3];

    for ch in 0..3 {
        let dmax = density_max[ch] + g.density_min[ch];
        let particle_area = g.agx_particle_area_um2 * g.agx_particle_scale[ch];
        let n_particles = (pixel_area / particle_area).max(1.0);
        let n_sub = if g.n_sub_layers > 1 { g.n_sub_layers } else { 1 };
        let n_particles_sub = n_particles / n_sub as f64;

        let density_ch: Vec<f64> = (0..n_px).map(|px| density_cmy[px * 3 + ch] + g.density_min[ch]).collect();

        let base_seed = g.fixed_seed.unwrap_or(ch as u64);

        let mut result = vec![0.0f64; n_px];
        for sl in 0..n_sub {
            let seed = base_seed.wrapping_add((sl as u64) * 10);
            let sub_result = apply_grain_channel(&density_ch, dmax, n_particles_sub, g.uniformity[ch], seed, 0.0, width, height);
            for px in 0..n_px { result[px] += sub_result[px]; }
        }
        if n_sub > 1 {
            for px in 0..n_px { result[px] /= n_sub as f64; }
        }

        // Blur and subtract dmin
        if g.blur > 0.4 {
            result = gaussian_filter_2d(&result, width, height, g.blur);
        }
        for px in 0..n_px {
            out[px * 3 + ch] = result[px] - g.density_min[ch];
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Sublayer grain
// ---------------------------------------------------------------------------

fn apply_grain_sublayers(
    density_cmy: &[f64],
    width: usize,
    height: usize,
    pixel_size_um: f64,
    g: &GrainParams,
    density_curves_norm: &[f64],
    density_curves_layers: &[f64],
    n_le: usize,
    profile_type: &str,
) -> Vec<f64> {
    let n_px = width * height;
    let positive = profile_type == "positive";

    // Interpolate into sublayer densities
    let density_layers = interp_density_cmy_layers(
        density_cmy, density_curves_norm, density_curves_layers, positive, n_le,
    );
    // density_layers: [n_px * 9], layout [px, layer, ch]

    // Max density per sublayer per channel
    let mut density_max_layers = [[0.0f64; 3]; 3]; // [layer][ch]
    for le in 0..n_le {
        for layer in 0..3 {
            for ch in 0..3 {
                let v = density_curves_layers[le * 9 + layer * 3 + ch];
                if v.is_finite() && v > density_max_layers[layer][ch] {
                    density_max_layers[layer][ch] = v;
                }
            }
        }
    }
    let density_max_total: [f64; 3] = [
        density_max_layers[0][0] + density_max_layers[1][0] + density_max_layers[2][0],
        density_max_layers[0][1] + density_max_layers[1][1] + density_max_layers[2][1],
        density_max_layers[0][2] + density_max_layers[1][2] + density_max_layers[2][2],
    ];

    let pixel_area = pixel_size_um * pixel_size_um;
    let mut out_ch = vec![0.0f64; n_px * 3];

    for ch in 0..3usize {
        let mut ch_accum = vec![0.0f64; n_px];
        for layer in 0..3usize {
            let frac = if density_max_total[ch] > 1e-10 {
                density_max_layers[layer][ch] / density_max_total[ch]
            } else { 1.0 / 3.0 };
            let dmin_layer = frac * g.density_min[ch];
            let dmax_layer = density_max_layers[layer][ch] + dmin_layer;
            let particle_area = g.agx_particle_area_um2
                * g.agx_particle_scale[ch]
                * g.agx_particle_scale_layers[layer];
            let n_particles = (pixel_area * frac / particle_area.max(1e-15)).max(1.0);

            let density_slice: Vec<f64> = (0..n_px)
                .map(|px| density_layers[px * 9 + layer * 3 + ch] + dmin_layer)
                .collect();
            let base_seed = g.fixed_seed.unwrap_or(ch as u64);
            let seed = base_seed.wrapping_add((layer as u64) * 10);
            // dye-cloud blur in pixels
            let dye_blur_px = g.blur_dye_clouds_um / pixel_size_um;
            let layer_out = apply_grain_channel(
                &density_slice, dmax_layer, n_particles, g.uniformity[ch], seed, dye_blur_px, width, height,
            );
            for px in 0..n_px { ch_accum[px] += layer_out[px]; }
        }
        // Micro-structure
        let ch_accum = add_micro_structure(ch_accum, g.micro_structure, pixel_size_um, width, height);
        // Subtract dmin, apply final blur
        let mut final_out: Vec<f64> = ch_accum.into_iter().map(|v| v - g.density_min[ch]).collect();
        if g.blur > 0.4 {
            final_out = gaussian_filter_2d(&final_out, width, height, g.blur);
        }
        for px in 0..n_px { out_ch[px * 3 + ch] = final_out[px]; }
    }
    out_ch
}

fn add_micro_structure(mut data: Vec<f64>, micro: (f64, f64), pixel_size_um: f64, width: usize, height: usize) -> Vec<f64> {
    let sigma_nm = micro.1;
    let sigma_px = sigma_nm * 0.001 / pixel_size_um;
    if sigma_px > 0.05 {
        // Lognormal clumping field with mean ≈ 1.0 and tunable roughness.
        let n = data.len();
        let mut rng = SmallRng::seed_from_u64(42);
        let mut clump: Vec<f64> = (0..n).map(|_| fast_lognormal_from_mean_std_scalar(1.0, sigma_px, &mut rng)).collect();
        let blur_px = micro.0 / pixel_size_um;
        if blur_px > 0.4 {
            clump = gaussian_filter_2d(&clump, width, height, blur_px);
        }
        for (v, c) in data.iter_mut().zip(clump.iter()) { *v *= c; }
    }
    data
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn apply_grain(
    density_cmy: &[f64],
    pixel_size_um: f64,
    grain: &GrainParams,
    density_curves_norm: &[f64],
    density_curves_layers: &[f64],
    n_le: usize,
    profile_type: &str,
    shape: (usize, usize),
) -> Vec<f64> {
    if !grain.active {
        return density_cmy.to_vec();
    }
    let (h, w) = shape;
    if grain.sublayers_active && !density_curves_layers.is_empty() {
        apply_grain_sublayers(density_cmy, w, h, pixel_size_um, grain,
                              density_curves_norm, density_curves_layers, n_le, profile_type)
    } else {
        apply_grain_simple(density_cmy, w, h, pixel_size_um, grain, density_curves_norm, n_le)
    }
}
