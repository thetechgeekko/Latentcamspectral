//! Print/paper glare model.
//!
//! Mirrors `spektrafilm/model/glare.py`.

use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand_distr::{Distribution, LogNormal};

use crate::utils::gaussian::gaussian_filter_2d;

#[derive(Debug, Clone)]
pub struct GlareParams {
    pub active: bool,
    pub percent: f64,
    pub roughness: f64,
    pub blur: f64,
}

impl Default for GlareParams {
    fn default() -> Self {
        Self { active: true, percent: 0.03, roughness: 0.7, blur: 0.5 }
    }
}

/// Add stochastic glare to an XYZ image.
///
/// `xyz` : flat [N_PX × 3].
/// `illuminant_xyz` : [3] XYZ of the viewing illuminant.
/// Modified in-place.
pub fn add_glare(xyz: &mut [f64], illuminant_xyz: &[f64; 3], glare: &GlareParams, width: usize, height: usize) {
    if !glare.active || glare.percent <= 0.0 { return; }

    let n_px = width * height;
    let amount = glare.percent;
    let roughness = glare.roughness;

    // Lognormal random field with mean = amount
    let sigma_ln = (1.0 + (roughness * amount / amount).powi(2)).ln().sqrt();
    let mu_ln = amount.ln() - 0.5 * sigma_ln * sigma_ln;

    let ln = LogNormal::new(mu_ln, sigma_ln).unwrap_or_else(|_| LogNormal::new(0.0, 0.1).unwrap());
    let mut rng = SmallRng::seed_from_u64(99);
    let mut field: Vec<f64> = (0..n_px).map(|_| ln.sample(&mut rng)).collect();

    if glare.blur > 0.1 {
        field = gaussian_filter_2d(&field, width, height, glare.blur);
    }

    // Scale to percent (field already has mean ≈ amount)
    let scale = 1.0 / 100.0;
    for px in 0..n_px {
        let g = field[px] * scale;
        for ch in 0..3 {
            xyz[px * 3 + ch] += g * illuminant_xyz[ch];
        }
    }
}
