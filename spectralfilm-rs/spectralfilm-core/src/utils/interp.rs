//! 1-D and N-D interpolation utilities.

/// Linear interpolation with clamped extrapolation on uniform or non-uniform grids.
///
/// `xs` must be monotonically increasing.
#[inline]
pub fn linear_interp_clamped(x: f64, xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len();
    if n == 0 { return 0.0; }
    if x <= xs[0] { return ys[0]; }
    if x >= xs[n - 1] { return ys[n - 1]; }
    // Binary search
    let mut lo = 0usize;
    let mut hi = n - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if xs[mid] <= x { lo = mid; } else { hi = mid; }
    }
    let dx = xs[hi] - xs[lo];
    if dx.abs() < 1e-30 { return ys[lo]; }
    let t = (x - xs[lo]) / dx;
    ys[lo] + t * (ys[hi] - ys[lo])
}

/// Apply 1-D interpolation to every element of `data` using pre-built `xs`/`ys` tables.
pub fn interp_vec(data: &[f64], xs: &[f64], ys: &[f64]) -> Vec<f64> {
    data.iter().map(|&x| linear_interp_clamped(x, xs, ys)).collect()
}
