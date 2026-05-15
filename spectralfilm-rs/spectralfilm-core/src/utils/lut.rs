//! 3-D LUT for spectral integral acceleration.
//!
//! Mirrors `spektrafilm/utils/lut.py` and the PCHIP/Mitchell logic in
//! `fast_interp_lut.py`. Here we use trilinear interpolation (adequate for
//! smoothly-varying spectral integrals) with optional tetrahedral lookup.

use rayon::prelude::*;

/// A 3-D LUT mapping CMY density → output (e.g. log-XYZ or log-raw).
///
/// Layout: `data[r * steps^2 * n_out + g * steps * n_out + b * n_out + out_ch]`.
#[derive(Debug, Clone)]
pub struct Lut3d {
    pub steps: usize,
    pub n_out: usize,
    /// `xmin[3]`, `xmax[3]` define the input domain per channel.
    pub xmin: [f64; 3],
    pub xmax: [f64; 3],
    pub data: Vec<f64>,
}

impl Lut3d {
    /// Build a 3-D LUT by evaluating `f` on a regular grid.
    ///
    /// `f` takes a flat batch `[N × 3]` input and returns a flat `[N × n_out]` output.
    pub fn build<F>(steps: usize, xmin: [f64; 3], xmax: [f64; 3], n_out: usize, f: &F) -> Self
    where
        F: Fn(&[f64]) -> Vec<f64> + Send + Sync,
    {
        let total = steps * steps * steps;
        // Build grid points
        let mut grid = vec![0.0f64; total * 3];
        for r in 0..steps {
            let x0 = xmin[0] + r as f64 * (xmax[0] - xmin[0]) / (steps - 1).max(1) as f64;
            for g in 0..steps {
                let x1 = xmin[1] + g as f64 * (xmax[1] - xmin[1]) / (steps - 1).max(1) as f64;
                for b in 0..steps {
                    let x2 = xmin[2] + b as f64 * (xmax[2] - xmin[2]) / (steps - 1).max(1) as f64;
                    let idx = (r * steps * steps + g * steps + b) * 3;
                    grid[idx] = x0;
                    grid[idx + 1] = x1;
                    grid[idx + 2] = x2;
                }
            }
        }
        let values = f(&grid);
        Self { steps, n_out, xmin, xmax, data: values }
    }

    /// Trilinear lookup for a single input point `[3]`. Returns `[n_out]`.
    pub fn lookup(&self, x: &[f64; 3]) -> Vec<f64> {
        let s = self.steps;
        let nout = self.n_out;
        // Normalise to [0, steps-1]
        let f = |i: usize| -> (usize, usize, f64) {
            let lo = ((x[i] - self.xmin[i]) / (self.xmax[i] - self.xmin[i]).max(1e-30)
                * (s - 1) as f64).floor() as isize;
            let lo = lo.clamp(0, s as isize - 2) as usize;
            let hi = (lo + 1).min(s - 1);
            let t = ((x[i] - self.xmin[i]) / (self.xmax[i] - self.xmin[i]).max(1e-30)
                * (s - 1) as f64) - lo as f64;
            (lo, hi, t.clamp(0.0, 1.0))
        };
        let (r0, r1, tr) = f(0);
        let (g0, g1, tg) = f(1);
        let (b0, b1, tb) = f(2);

        let at = |r, g, b, ch| self.data[(r * s * s + g * s + b) * nout + ch];
        let mut out = vec![0.0f64; nout];
        for ch in 0..nout {
            // Trilinear interpolation
            let c000 = at(r0, g0, b0, ch);
            let c001 = at(r0, g0, b1, ch);
            let c010 = at(r0, g1, b0, ch);
            let c011 = at(r0, g1, b1, ch);
            let c100 = at(r1, g0, b0, ch);
            let c101 = at(r1, g0, b1, ch);
            let c110 = at(r1, g1, b0, ch);
            let c111 = at(r1, g1, b1, ch);
            let c00 = c000 * (1.0 - tb) + c001 * tb;
            let c01 = c010 * (1.0 - tb) + c011 * tb;
            let c10 = c100 * (1.0 - tb) + c101 * tb;
            let c11 = c110 * (1.0 - tb) + c111 * tb;
            let c0 = c00 * (1.0 - tg) + c01 * tg;
            let c1 = c10 * (1.0 - tg) + c11 * tg;
            out[ch] = c0 * (1.0 - tr) + c1 * tr;
        }
        out
    }

    /// Apply the LUT to a flat `[N × 3]` input, returning `[N × n_out]`.
    pub fn apply(&self, data: &[f64]) -> Vec<f64> {
        let n_px = data.len() / 3;
        let nout = self.n_out;
        let mut out = vec![0.0f64; n_px * nout];
        out.par_chunks_mut(nout).enumerate().for_each(|(px, chunk)| {
            let x = [data[px * 3], data[px * 3 + 1], data[px * 3 + 2]];
            let v = self.lookup(&x);
            chunk.copy_from_slice(&v);
        });
        out
    }
}
