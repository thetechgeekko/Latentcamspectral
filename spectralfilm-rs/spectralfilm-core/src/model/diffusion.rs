//! Halation, diffusion filter, lens blur.
//!
//! Mirrors `spektrafilm/model/diffusion.py`.

use crate::utils::gaussian::{gaussian_filter_2d, gaussian_filter_2d_ch};

// ---------------------------------------------------------------------------
// Parameter structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct HalationParams {
    pub active: bool,
    pub scatter_amount: f64,
    pub scatter_spatial_scale: f64,
    pub halation_amount: f64,
    pub halation_spatial_scale: f64,
    pub scatter_core_um: [f64; 3],
    pub scatter_tail_um: [f64; 3],
    pub scatter_tail_weight: [f64; 3],
    pub boost_ev: f64,
    pub boost_range: f64,
    pub protect_ev: f64,
    pub halation_strength: [f64; 3],
    pub halation_first_sigma_um: [f64; 3],
    pub halation_n_bounces: usize,
    pub halation_bounce_decay: f64,
    pub halation_renormalize: bool,
}

impl Default for HalationParams {
    fn default() -> Self {
        Self {
            active: true,
            scatter_amount: 1.0,
            scatter_spatial_scale: 1.0,
            halation_amount: 1.0,
            halation_spatial_scale: 1.0,
            scatter_core_um: [2.6, 2.3, 1.8],
            scatter_tail_um: [8.8, 7.0, 6.4],
            scatter_tail_weight: [0.74, 0.64, 0.64],
            boost_ev: 0.0,
            boost_range: 0.3,
            protect_ev: 4.0,
            halation_strength: [0.05, 0.015, 0.0],
            halation_first_sigma_um: [65.0, 65.0, 65.0],
            halation_n_bounces: 3,
            halation_bounce_decay: 0.5,
            halation_renormalize: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiffusionFilterParams {
    pub active: bool,
    pub filter_family: String,
    pub strength: f64,
    pub spatial_scale: f64,
    pub halo_warmth: f64,
    pub core_intensity: f64,
    pub core_size: f64,
    pub halo_intensity: f64,
    pub halo_size: f64,
    pub bloom_intensity: f64,
    pub bloom_size: f64,
}

impl Default for DiffusionFilterParams {
    fn default() -> Self {
        Self {
            active: false,
            filter_family: "black_pro_mist".to_string(),
            strength: 0.5,
            spatial_scale: 1.0,
            halo_warmth: 0.0,
            core_intensity: 1.0,
            core_size: 1.0,
            halo_intensity: 1.0,
            halo_size: 1.0,
            bloom_intensity: 1.0,
            bloom_size: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Highlight boost
// ---------------------------------------------------------------------------

/// Reconstruct pre-clip irradiance in highlight regions before propagation.
pub fn boost_highlights(raw: &mut [f64], boost_ev: f64, boost_range: f64, protect_ev: f64) {
    if boost_ev <= 0.0 { return; }
    let boost_linear = 2.0_f64.powf(boost_ev);
    for v in raw.iter_mut() {
        if !v.is_finite() { continue; }
        // Soft transition in log-exposure space around protect_ev
        let t = ((*v / 2.0_f64.powf(protect_ev) - 1.0) / boost_range.max(1e-6)).clamp(0.0, 1.0);
        let smooth = t * t * (3.0 - 2.0 * t); // smoothstep
        *v *= 1.0 + smooth * (boost_linear - 1.0);
    }
}

// ---------------------------------------------------------------------------
// Gaussian blur (pixel units)
// ---------------------------------------------------------------------------

pub fn apply_gaussian_blur(image: &mut Vec<f64>, width: usize, height: usize, n_ch: usize, sigma_px: f64) {
    if sigma_px < 0.01 { return; }
    *image = gaussian_filter_2d_ch(image, width, height, n_ch, sigma_px);
}

/// Gaussian blur in physical units (µm).
pub fn apply_gaussian_blur_um(image: &mut Vec<f64>, width: usize, height: usize, n_ch: usize, sigma_um: f64, pixel_size_um: f64) {
    if pixel_size_um <= 0.0 || sigma_um <= 0.0 { return; }
    apply_gaussian_blur(image, width, height, n_ch, sigma_um / pixel_size_um);
}

// ---------------------------------------------------------------------------
// Halation (scatter + back-reflection)
// ---------------------------------------------------------------------------

/// Apply in-emulsion scatter and back-reflection halation.
///
/// `raw` is the linear irradiance image, shape [N_PX × 3] (flat, row-major).
/// Modified in-place; returns the updated buffer.
pub fn apply_halation_um(
    raw: &[f64],
    params: &HalationParams,
    pixel_size_um: f64,
    width: usize,
    height: usize,
) -> Vec<f64> {
    if !params.active || pixel_size_um <= 0.0 {
        return raw.to_vec();
    }
    let n_px = width * height;
    let mut out = raw.to_vec();

    for ch in 0..3usize {
        // --- In-emulsion scatter ---
        let core_um = params.scatter_core_um[ch] * params.scatter_spatial_scale * params.scatter_amount;
        let tail_um = params.scatter_tail_um[ch] * params.scatter_spatial_scale * params.scatter_amount;
        let tail_w  = params.scatter_tail_weight[ch];

        let core_px = core_um / pixel_size_um;
        let tail_px = tail_um / pixel_size_um;

        let slice: Vec<f64> = (0..n_px).map(|px| raw[px * 3 + ch]).collect();

        // Energy-preserving mixture: (1-w)*Gaussian_core + w*Gaussian_tail
        let scattered = if core_px > 0.05 || tail_px > 0.05 {
            let core_blur = if core_px > 0.05 { gaussian_filter_2d(&slice, width, height, core_px) } else { slice.clone() };
            let tail_blur = if tail_px > 0.05 { gaussian_filter_2d(&slice, width, height, tail_px) } else { slice.clone() };
            let mut sc = vec![0.0f64; n_px];
            for px in 0..n_px {
                sc[px] = (1.0 - tail_w) * core_blur[px] + tail_w * tail_blur[px];
            }
            sc
        } else {
            slice.clone()
        };

        for px in 0..n_px { out[px * 3 + ch] = scattered[px]; }

        // --- Back-reflection halation ---
        let strength = params.halation_strength[ch] * params.halation_amount;
        if strength <= 0.0 { continue; }

        let sigma0_px = params.halation_first_sigma_um[ch] * params.halation_spatial_scale / pixel_size_um;
        for bounce in 0..params.halation_n_bounces {
            let sigma_px = sigma0_px * ((bounce + 1) as f64).sqrt();
            let decay = strength * params.halation_bounce_decay.powi(bounce as i32);
            if sigma_px < 0.5 { continue; }
            let halo = gaussian_filter_2d(&slice, width, height, sigma_px);
            for px in 0..n_px {
                out[px * 3 + ch] += decay * halo[px];
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Diffusion filter (Black Pro-Mist, Pro-Mist, Glimmerglass, CineBloom)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct DiffusionGroupCfg {
    lambda_um: f64,
    spread: f64,
    n_components: usize,
    alpha: f64,
}

#[derive(Debug, Clone, Copy)]
struct DiffusionFamilyCfg {
    core: DiffusionGroupCfg,
    halo: DiffusionGroupCfg,
    bloom: DiffusionGroupCfg,
    w_c: f64,
    w_h: f64,
    w_b: f64,
    halo_warmth_base: f64,
    total_gain: f64,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedDiffusionFamilyCfg {
    core: DiffusionGroupCfg,
    halo: DiffusionGroupCfg,
    bloom: DiffusionGroupCfg,
    w_c: f64,
    w_h: f64,
    w_b: f64,
    halo_warmth_base: f64,
    total_gain: f64,
}

/// Analytic radial profile of the diffusion filter PSF.
///
/// Units are 1 / µm² when produced by `diffusion_filter_radial_profile`.
#[derive(Debug, Clone)]
pub struct DiffusionFilterRadialProfile {
    pub core: Vec<f64>,
    pub halo: [Vec<f64>; 3],
    pub bloom: Vec<f64>,
    pub total_per_channel: [Vec<f64>; 3],
}

const HALO_CHANNEL_WARMTH_AXIS: [f64; 3] = [1.30, 0.15, -1.45];
const STRENGTH_BREAKPOINTS: [f64; 5] = [0.125, 0.25, 0.5, 1.0, 2.0];
const STRENGTH_TOTAL_FRACTION: [f64; 5] = [0.10, 0.20, 0.35, 0.55, 0.75];

const GLIMMERGLASS_CFG: DiffusionFamilyCfg = DiffusionFamilyCfg {
    core: DiffusionGroupCfg { lambda_um: 10.0, spread: 1.5, n_components: 2, alpha: 3.0 },
    halo: DiffusionGroupCfg { lambda_um: 50.0, spread: 2.0, n_components: 3, alpha: 3.0 },
    bloom: DiffusionGroupCfg { lambda_um: 260.0, spread: 2.5, n_components: 4, alpha: 3.2 },
    w_c: 0.60, w_h: 0.30, w_b: 0.10,
    halo_warmth_base: 0.0,
    total_gain: 0.65,
};

const BLACK_PRO_MIST_CFG: DiffusionFamilyCfg = DiffusionFamilyCfg {
    core: DiffusionGroupCfg { lambda_um: 16.0, spread: 1.5, n_components: 2, alpha: 3.0 },
    halo: DiffusionGroupCfg { lambda_um: 95.0, spread: 2.0, n_components: 3, alpha: 3.0 },
    bloom: DiffusionGroupCfg { lambda_um: 380.0, spread: 2.5, n_components: 4, alpha: 3.5 },
    w_c: 0.40, w_h: 0.47, w_b: 0.13,
    halo_warmth_base: 0.65,
    total_gain: 0.75,
};

const PRO_MIST_CFG: DiffusionFamilyCfg = DiffusionFamilyCfg {
    core: DiffusionGroupCfg { lambda_um: 14.0, spread: 1.5, n_components: 2, alpha: 3.0 },
    halo: DiffusionGroupCfg { lambda_um: 150.0, spread: 2.0, n_components: 3, alpha: 3.0 },
    bloom: DiffusionGroupCfg { lambda_um: 650.0, spread: 2.5, n_components: 4, alpha: 2.9 },
    w_c: 0.28, w_h: 0.42, w_b: 0.30,
    halo_warmth_base: 0.40,
    total_gain: 1.05,
};

const CINEBLOOM_CFG: DiffusionFamilyCfg = DiffusionFamilyCfg {
    core: DiffusionGroupCfg { lambda_um: 20.0, spread: 1.5, n_components: 2, alpha: 3.0 },
    halo: DiffusionGroupCfg { lambda_um: 200.0, spread: 2.0, n_components: 3, alpha: 3.0 },
    bloom: DiffusionGroupCfg { lambda_um: 1000.0, spread: 2.5, n_components: 4, alpha: 2.5 },
    w_c: 0.22, w_h: 0.30, w_b: 0.48,
    halo_warmth_base: 0.85,
    total_gain: 1.00,
};

/// Canonical diffusion-filter families ported from Python.
pub const DIFFUSION_FILTER_FAMILIES: [&str; 4] = [
    "glimmerglass",
    "black_pro_mist",
    "pro_mist",
    "cinebloom",
];

fn family_cfg(family: &str) -> DiffusionFamilyCfg {
    match family {
        "glimmerglass" => GLIMMERGLASS_CFG,
        "black_pro_mist" => BLACK_PRO_MIST_CFG,
        "pro_mist" => PRO_MIST_CFG,
        "cinebloom" => CINEBLOOM_CFG,
        // Backwards-compatible aliases for names that existed in the prior
        // Rust approximation but are not distinct families in the Python port.
        "white_pro_mist" => PRO_MIST_CFG,
        "hollywood_black" => CINEBLOOM_CFG,
        "tiffen_soft_fx" => GLIMMERGLASS_CFG,
        _ => BLACK_PRO_MIST_CFG,
    }
}

fn resolve_family_cfg(family: &str, params: Option<&DiffusionFilterParams>) -> ResolvedDiffusionFamilyCfg {
    let base = family_cfg(family);
    let mut out = ResolvedDiffusionFamilyCfg {
        core: base.core,
        halo: base.halo,
        bloom: base.bloom,
        w_c: base.w_c,
        w_h: base.w_h,
        w_b: base.w_b,
        halo_warmth_base: base.halo_warmth_base,
        total_gain: base.total_gain,
    };

    if let Some(p) = params {
        let w_c = base.w_c * p.core_intensity.max(0.0);
        let w_h = base.w_h * p.halo_intensity.max(0.0);
        let w_b = base.w_b * p.bloom_intensity.max(0.0);
        let total = w_c + w_h + w_b;
        if total > 0.0 {
            out.core.lambda_um = base.core.lambda_um * p.core_size.max(1e-6);
            out.halo.lambda_um = base.halo.lambda_um * p.halo_size.max(1e-6);
            out.bloom.lambda_um = base.bloom.lambda_um * p.bloom_size.max(1e-6);
            out.w_c = w_c / total;
            out.w_h = w_h / total;
            out.w_b = w_b / total;
        }
    }

    out
}

fn strength_to_scatter(strength: f64, family: &str) -> f64 {
    if strength <= 0.0 || !strength.is_finite() {
        return 0.0;
    }

    let log_strength = strength.max(1e-6).log2();
    let first_log = STRENGTH_BREAKPOINTS[0].log2();
    let last_log = STRENGTH_BREAKPOINTS[STRENGTH_BREAKPOINTS.len() - 1].log2();
    let base_total = if log_strength <= first_log {
        STRENGTH_TOTAL_FRACTION[0]
    } else if log_strength >= last_log {
        STRENGTH_TOTAL_FRACTION[STRENGTH_TOTAL_FRACTION.len() - 1]
    } else {
        let mut value = STRENGTH_TOTAL_FRACTION[0];
        for i in 0..(STRENGTH_BREAKPOINTS.len() - 1) {
            let x0 = STRENGTH_BREAKPOINTS[i].log2();
            let x1 = STRENGTH_BREAKPOINTS[i + 1].log2();
            if log_strength >= x0 && log_strength <= x1 {
                let t = (log_strength - x0) / (x1 - x0);
                value = STRENGTH_TOTAL_FRACTION[i] * (1.0 - t) + STRENGTH_TOTAL_FRACTION[i + 1] * t;
                break;
            }
        }
        value
    };

    (base_total * family_cfg(family).total_gain).clamp(0.0, 0.99)
}

fn expand_group(group_cfg: DiffusionGroupCfg, kind: &str) -> (Vec<f64>, Vec<f64>) {
    let lambda_center = group_cfg.lambda_um;
    let spread = group_cfg.spread;
    let n = group_cfg.n_components.max(1);

    if n == 1 || spread <= 1.0 {
        return (vec![lambda_center], vec![1.0]);
    }

    let log_lo = (lambda_center / spread).ln();
    let log_hi = (lambda_center * spread).ln();
    let denom = (n - 1).max(1) as f64;
    let lambdas: Vec<f64> = (0..n)
        .map(|i| (log_lo + (log_hi - log_lo) * (i as f64 / denom)).exp())
        .collect();

    let mut weights: Vec<f64> = if kind == "bloom" {
        let alpha = group_cfg.alpha;
        lambdas.iter().map(|lambda| lambda.powf(2.0 - alpha)).collect()
    } else {
        vec![1.0; n]
    };

    let sum: f64 = weights.iter().sum();
    if sum > 0.0 {
        for w in &mut weights { *w /= sum; }
    } else {
        let uniform = 1.0 / n as f64;
        weights.fill(uniform);
    }

    (lambdas, weights)
}

fn halo_channel_weights(weights: &[f64], warmth: f64) -> [Vec<f64>; 3] {
    let n = weights.len();
    if n < 2 {
        return [weights.to_vec(), weights.to_vec(), weights.to_vec()];
    }

    let warmth = warmth.clamp(-1.5, 1.5);
    let target_total: f64 = weights.iter().sum();
    let denom = (n - 1) as f64;
    let mut gradient: Vec<f64> = (0..n).map(|i| -1.0 + 2.0 * (i as f64 / denom)).collect();
    let weighted_mean = gradient.iter().zip(weights.iter()).map(|(g, w)| g * w).sum::<f64>() / target_total.max(1e-12);
    for g in &mut gradient { *g -= weighted_mean; }

    let mut out = [Vec::new(), Vec::new(), Vec::new()];
    for c in 0..3 {
        let mut raw: Vec<f64> = weights.iter()
            .zip(gradient.iter())
            .map(|(w, g)| (w * (1.0 + warmth * HALO_CHANNEL_WARMTH_AXIS[c] * g)).max(0.0))
            .collect();
        let sum: f64 = raw.iter().sum();
        if sum > 0.0 {
            for w in &mut raw { *w *= target_total / sum; }
            out[c] = raw;
        } else {
            out[c] = weights.to_vec();
        }
    }
    out
}

fn exp_radial_sum(radius: &[f64], lambdas: &[f64], weights: &[f64]) -> Vec<f64> {
    let mut total = vec![0.0; radius.len()];
    for (&weight, &lambda) in weights.iter().zip(lambdas.iter()) {
        let lambda = lambda.max(1e-6);
        let norm = 1.0 / (2.0 * std::f64::consts::PI * lambda * lambda);
        for (i, &r) in radius.iter().enumerate() {
            total[i] += weight * (-r / lambda).exp() * norm;
        }
    }
    total
}

fn bloom_max_lambda_um(family: &str, params: Option<&DiffusionFilterParams>) -> f64 {
    let cfg = resolve_family_cfg(family, params);
    cfg.bloom.lambda_um * cfg.bloom.spread
}

/// Analytic radial profile of the diffusion-filter PSF, unit-normalised in 2D.
///
/// Returns each component contribution in 1 / µm². `halo_warmth` is added to
/// the per-family base, matching the Python model. Unknown family names fall
/// back to Black Pro-Mist to preserve the previous Rust runtime behaviour.
pub fn diffusion_filter_radial_profile(
    radius_um: &[f64],
    family: &str,
    spatial_scale: f64,
    halo_warmth: f64,
    params: Option<&DiffusionFilterParams>,
) -> DiffusionFilterRadialProfile {
    let cfg = resolve_family_cfg(family, params);
    let spatial_scale = spatial_scale.max(1e-6);
    let effective_warmth = cfg.halo_warmth_base + halo_warmth;

    let (core_lambdas, core_weights) = expand_group(cfg.core, "core");
    let (halo_lambdas, halo_weights) = expand_group(cfg.halo, "halo");
    let (bloom_lambdas, bloom_weights) = expand_group(cfg.bloom, "bloom");
    let halo_per_ch = halo_channel_weights(&halo_weights, effective_warmth);

    let scale_lambdas = |lambdas: &[f64]| -> Vec<f64> { lambdas.iter().map(|v| v * spatial_scale).collect() };
    let core_lambdas_um = scale_lambdas(&core_lambdas);
    let halo_lambdas_um = scale_lambdas(&halo_lambdas);
    let bloom_lambdas_um = scale_lambdas(&bloom_lambdas);

    let mut core = exp_radial_sum(radius_um, &core_lambdas_um, &core_weights);
    let mut bloom = exp_radial_sum(radius_um, &bloom_lambdas_um, &bloom_weights);
    for v in &mut core { *v *= cfg.w_c; }
    for v in &mut bloom { *v *= cfg.w_b; }

    let mut halo = [Vec::new(), Vec::new(), Vec::new()];
    let mut total_per_channel = [Vec::new(), Vec::new(), Vec::new()];
    for c in 0..3 {
        halo[c] = exp_radial_sum(radius_um, &halo_lambdas_um, &halo_per_ch[c]);
        for v in &mut halo[c] { *v *= cfg.w_h; }
        total_per_channel[c] = (0..radius_um.len())
            .map(|i| core[i] + halo[c][i] + bloom[i])
            .collect();
    }

    DiffusionFilterRadialProfile { core, halo, bloom, total_per_channel }
}

/// Per-channel 2D PSF for a diffusion filter.
///
/// Returns a flat `[height × width × 3]` buffer. Each channel is normalised to
/// sum to one on the discrete grid. The radial profile itself is the exact
/// Python exponential-mixture profile; only the optional application fast path
/// below approximates convolution for performance.
pub fn diffusion_filter_psf(
    width: usize,
    height: usize,
    family: &str,
    spatial_scale: f64,
    pixel_size_um: f64,
    halo_warmth: f64,
    params: Option<&DiffusionFilterParams>,
) -> Vec<f64> {
    if width == 0 || height == 0 || pixel_size_um <= 0.0 {
        return Vec::new();
    }

    let cx = width / 2;
    let cy = height / 2;
    let mut radius_px = vec![0.0; width * height];
    for y in 0..height {
        for x in 0..width {
            let dx = x as isize - cx as isize;
            let dy = y as isize - cy as isize;
            radius_px[y * width + x] = ((dx * dx + dy * dy) as f64).sqrt();
        }
    }

    let cfg = resolve_family_cfg(family, params);
    let effective_warmth = cfg.halo_warmth_base + halo_warmth;
    let spatial_scale = spatial_scale.max(1e-6);

    let (core_lambdas, core_weights) = expand_group(cfg.core, "core");
    let (halo_lambdas, halo_weights) = expand_group(cfg.halo, "halo");
    let (bloom_lambdas, bloom_weights) = expand_group(cfg.bloom, "bloom");
    let halo_per_ch = halo_channel_weights(&halo_weights, effective_warmth);

    let to_px = |lambdas: &[f64]| -> Vec<f64> {
        lambdas.iter().map(|v| v * spatial_scale / pixel_size_um).collect()
    };
    let core_lambdas_px = to_px(&core_lambdas);
    let halo_lambdas_px = to_px(&halo_lambdas);
    let bloom_lambdas_px = to_px(&bloom_lambdas);

    let mut core = exp_radial_sum(&radius_px, &core_lambdas_px, &core_weights);
    let mut bloom = exp_radial_sum(&radius_px, &bloom_lambdas_px, &bloom_weights);
    for v in &mut core { *v *= cfg.w_c; }
    for v in &mut bloom { *v *= cfg.w_b; }

    let n_px = width * height;
    let mut psf = vec![0.0; n_px * 3];
    for c in 0..3 {
        let mut halo = exp_radial_sum(&radius_px, &halo_lambdas_px, &halo_per_ch[c]);
        for v in &mut halo { *v *= cfg.w_h; }

        let mut sum = 0.0;
        for i in 0..n_px {
            let v = core[i] + halo[i] + bloom[i];
            psf[i * 3 + c] = v;
            sum += v;
        }
        if sum > 0.0 {
            for i in 0..n_px {
                psf[i * 3 + c] /= sum;
            }
        }
    }

    psf
}

/// Apply a diffusion filter in physical units.
///
/// This follows the Python model's family configuration, strength-to-scatter
/// mapping, core/halo/bloom expansion, and energy-conserving halo warmth
/// redistribution. The Python implementation uses FFT convolution with the
/// exact exponential-mixture PSF. This Rust core currently has only Gaussian
/// convolution utilities available in scope, so each 2D exponential component
/// is approximated during application by a moment-matched Gaussian
/// (`sigma = sqrt(3) * lambda`). The exported `diffusion_filter_psf` and
/// `diffusion_filter_radial_profile` functions still generate the exact
/// exponential-mixture profile for inspection or future convolution backends.
pub fn apply_diffusion_filter_um(
    image: &[f64],
    params: &DiffusionFilterParams,
    pixel_size_um: f64,
    width: usize,
    height: usize,
    n_ch: usize,
) -> Vec<f64> {
    if !params.active
        || params.strength <= 0.0
        || params.spatial_scale <= 0.0
        || pixel_size_um <= 0.0
        || width == 0
        || height == 0
        || n_ch == 0
    {
        return image.to_vec();
    }

    let family = params.filter_family.as_str();
    let p_scatter = strength_to_scatter(params.strength, family);
    if p_scatter <= 0.0 {
        return image.to_vec();
    }

    let psf = diffusion_filter_psf(
        width,
        height,
        family,
        params.spatial_scale,
        pixel_size_um,
        params.halo_warmth,
        Some(params),
    );

    let n_px = width * height;
    let mut out = vec![0.0; image.len()];

    for ch in 0..n_ch {
        let slice: Vec<f64> = (0..n_px).map(|px| image[px * n_ch + ch]).collect();
        let psf_ch = ch.min(2);
        let psf_slice: Vec<f64> = (0..n_px).map(|px| psf[px * 3 + psf_ch]).collect();

        let deflected = crate::utils::fft::convolve_2d_fft(&slice, &psf_slice, width, height);

        for px in 0..n_px {
            out[px * n_ch + ch] = (1.0 - p_scatter) * slice[px] + p_scatter * deflected[px];
        }
    }

    out
}

/// Returns (core_um, halo_um, bloom_um) central PSF widths for a diffusion
/// filter family. Kept for compatibility with older Rust callers.
fn family_psf_um(family: &str, scale: f64) -> (f64, f64, f64) {
    let cfg = resolve_family_cfg(family, None);
    (cfg.core.lambda_um * scale, cfg.halo.lambda_um * scale, cfg.bloom.lambda_um * scale)
}

// ---------------------------------------------------------------------------
// Unsharp mask
// ---------------------------------------------------------------------------

pub fn apply_unsharp_mask(
    image: &[f64],
    width: usize,
    height: usize,
    n_ch: usize,
    sigma: f64,
    amount: f64,
) -> Vec<f64> {
    let n_px = width * height;
    let blurred = gaussian_filter_2d_ch(image, width, height, n_ch, sigma);
    let mut out = vec![0.0f64; image.len()];
    for px in 0..n_px {
        for ch in 0..n_ch {
            let i = px * n_ch + ch;
            out[i] = image[i] + amount * (image[i] - blurred[i]);
        }
    }
    out
}
