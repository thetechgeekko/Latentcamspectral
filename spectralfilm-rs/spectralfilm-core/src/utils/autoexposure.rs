//! Auto-exposure metering.
//!
//! Mirrors `spektrafilm/utils/autoexposure.py`.


/// Measure the auto-exposure correction in EV needed to bring the image
/// luminance to 18% grey.
///
/// `image` : flat [H × W × 3] linear RGB (already decoded).
/// `method` : `"center_weighted"` (default) or `"median"`.
///
/// Returns the EV compensation (negative = darken, positive = brighten).
pub fn measure_autoexposure_ev(
    image: &[f64],
    width: usize,
    height: usize,
    method: &str,
) -> f64 {
    let n_px = width * height;
    // Approximate luminance via sRGB primaries (CIE Y channel)
    // Y ≈ 0.2126 R + 0.7152 G + 0.0722 B
    let luminance: Vec<f64> = (0..n_px)
        .map(|px| {
            let r = image[px * 3];
            let g = image[px * 3 + 1];
            let b = image[px * 3 + 2];
            0.2126 * r + 0.7152 * g + 0.0722 * b
        })
        .collect();

    let exposure = match method {
        "median" => {
            let mut sorted = luminance.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = sorted.len() / 2;
            sorted.get(mid).copied().unwrap_or(crate::config::MIDGRAY)
        }
        _ => {
            // Centre-weighted: Gaussian mask with σ ≈ 0.2 of image diagonal
            let sigma = 0.2_f64;
            let h_f = height as f64;
            let w_f = width as f64;
            let norm = h_f.max(w_f);
            let mut weight_sum = 0.0f64;
            let mut weighted = 0.0f64;
            for row in 0..height {
                let y = (row as f64 / h_f - 0.5) * (h_f / norm);
                for col in 0..width {
                    let x = (col as f64 / w_f - 0.5) * (w_f / norm);
                    let w = (-(x * x + y * y) / (2.0 * sigma * sigma)).exp();
                    weighted += w * luminance[row * width + col];
                    weight_sum += w;
                }
            }
            if weight_sum > 1e-30 { weighted / weight_sum } else { crate::config::MIDGRAY }
        }
    };

    let exposure = exposure.max(1e-10);
    let ev = -(exposure / crate::config::MIDGRAY).log2();
    if ev.is_finite() { ev } else { 0.0 }
}
