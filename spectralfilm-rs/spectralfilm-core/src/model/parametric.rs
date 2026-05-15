//! Parametric H&D density-curve model.
//!
//! Mirrors `spektrafilm/model/parametric.py`.

/// Generate CMY density curves from the parametric toe/linear/shoulder model.
///
/// `log_exposure` is the exposure axis of length `N`. All remaining arguments
/// are per-channel CMY parameters. The returned vector is row-major with shape
/// `[N × 3]`, i.e. `out[i * 3 + ch]`.
///
/// Formula, per channel:
///
/// `g*toe*log10(1 + 10^((loge - loge0)/toe))
///  - g*shoulder*log10(1 + 10^((loge - loge0 - dmax/g)/shoulder))`
pub fn parametric_density_curves_model(
    log_exposure: &[f64],
    gamma: [f64; 3],
    log_exposure_0: [f64; 3],
    density_max: [f64; 3],
    toe_size: [f64; 3],
    shoulder_size: [f64; 3],
) -> Vec<f64> {
    let mut density_curves = vec![0.0f64; log_exposure.len() * 3];

    for ch in 0..3 {
        let g = gamma[ch];
        let loge0 = log_exposure_0[ch];
        let dmax = density_max[ch];
        let toe = toe_size[ch];
        let shoulder = shoulder_size[ch];

        for (i, &loge) in log_exposure.iter().enumerate() {
            let toe_term = g * toe * (1.0 + 10.0_f64.powf((loge - loge0) / toe)).log10();
            let shoulder_term = g
                * shoulder
                * (1.0 + 10.0_f64.powf((loge - loge0 - dmax / g) / shoulder)).log10();

            density_curves[i * 3 + ch] = toe_term - shoulder_term;
        }
    }

    density_curves
}
