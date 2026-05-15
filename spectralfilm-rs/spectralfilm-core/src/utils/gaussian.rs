//! Fast separable Gaussian filter.
//!
//! Implements a separable IIR Gaussian (Young & van Vliet 2002) for large σ
//! and a direct FIR for small σ, both operating in-place on flat image buffers.

use rayon::prelude::*;

/// Apply a 2-D Gaussian blur to a single-channel flat image `[H × W]`.
pub fn gaussian_filter_2d(image: &[f64], width: usize, height: usize, sigma: f64) -> Vec<f64> {
    if sigma < 0.01 || width == 0 || height == 0 {
        return image.to_vec();
    }
    let mut buf = image.to_vec();
    if sigma < 3.0 {
        fir_gaussian_1ch(&mut buf, width, height, sigma);
    } else {
        iir_gaussian_1ch(&mut buf, width, height, sigma);
    }
    buf
}

/// Apply a 2-D Gaussian blur to a multi-channel flat image `[H × W × n_ch]`.
pub fn gaussian_filter_2d_ch(image: &[f64], width: usize, height: usize, n_ch: usize, sigma: f64) -> Vec<f64> {
    if sigma < 0.01 || width == 0 || height == 0 {
        return image.to_vec();
    }
    let n_px = width * height;
    let mut out = image.to_vec();
    // Process each channel independently
    let channels: Vec<Vec<f64>> = (0..n_ch)
        .into_par_iter()
        .map(|ch| {
            let slice: Vec<f64> = (0..n_px).map(|px| image[px * n_ch + ch]).collect();
            gaussian_filter_2d(&slice, width, height, sigma)
        })
        .collect();
    for ch in 0..n_ch {
        for px in 0..n_px {
            out[px * n_ch + ch] = channels[ch][px];
        }
    }
    out
}

// ---------------------------------------------------------------------------
// FIR (finite impulse response) Gaussian — separable, for small sigma
// ---------------------------------------------------------------------------

fn fir_gaussian_1ch(buf: &mut Vec<f64>, width: usize, height: usize, sigma: f64) {
    let radius = ((3.0 * sigma).ceil() as usize).min(128);
    let kernel = gaussian_kernel_1d(sigma, radius);
    let klen = kernel.len();
    let krad = klen / 2;

    // Horizontal pass
    let mut tmp = buf.clone();
    for row in 0..height {
        for col in 0..width {
            let mut acc = 0.0f64;
            for k in 0..klen {
                let c = (col as isize + k as isize - krad as isize).clamp(0, width as isize - 1) as usize;
                acc += kernel[k] * buf[row * width + c];
            }
            tmp[row * width + col] = acc;
        }
    }
    // Vertical pass
    for row in 0..height {
        for col in 0..width {
            let mut acc = 0.0f64;
            for k in 0..klen {
                let r = (row as isize + k as isize - krad as isize).clamp(0, height as isize - 1) as usize;
                acc += kernel[k] * tmp[r * width + col];
            }
            buf[row * width + col] = acc;
        }
    }
}

fn gaussian_kernel_1d(sigma: f64, radius: usize) -> Vec<f64> {
    let r = radius as isize;
    let mut k: Vec<f64> = (-r..=r).map(|x| (-0.5 * (x as f64 / sigma).powi(2)).exp()).collect();
    let sum: f64 = k.iter().sum();
    for v in k.iter_mut() { *v /= sum; }
    k
}

// ---------------------------------------------------------------------------
// IIR (infinite impulse response) Gaussian — Young & van Vliet 2002
// O(1) per pixel, σ-independent cost.
// ---------------------------------------------------------------------------

/// Compute Young-van Vliet IIR coefficients for a given σ.
fn yvv_coeffs(sigma: f64) -> (f64, f64, f64, f64) {
    // Approximation valid for σ ≥ 0.5
    let q = if sigma >= 2.5 {
        0.98711 * sigma - 0.96330
    } else {
        3.97156 - 4.14554 * (1.0 - 0.26891 * sigma).sqrt()
    };
    let b0 = 1.57825 + q * (2.44413 + q * (1.4281 + q * 0.422205));
    let b1 = q * (2.44413 + q * (2.8562 + q * 1.26661));
    let b2 = -(q * q) * (1.4281 + q * 1.26661);
    let b3 = (q * q * q) * 0.422205;
    let b_sum = b0 + b1 + b2 + b3;
    (b1 / b_sum, b2 / b_sum, b3 / b_sum, b0 / b_sum)
}

fn reflect_index(mut i: isize, n: isize) -> usize {
    if n == 1 { return 0; }
    if i < 0 {
        i = -1 - i;
    }
    let p = 2 * n;
    i %= p;
    if i >= n {
        (p - 1 - i) as usize
    } else {
        i as usize
    }
}

fn iir_pass_forward(line: &mut [f64], b1: f64, b2: f64, b3: f64, b0: f64, sigma: f64) {
    let n = line.len();
    if n == 0 { return; }
    
    let pad_len = ((8.0 * sigma).ceil() as usize).max(3);
    let mut hist = [0.0; 3];
    
    for i in (-(pad_len as isize))..0 {
        let idx = reflect_index(i, n as isize);
        let val = line[idx];
        let new_val = b0 * val + b1 * hist[2] + b2 * hist[1] + b3 * hist[0];
        hist[0] = hist[1]; hist[1] = hist[2]; hist[2] = new_val;
    }
    
    for i in 0..n {
        let val = line[i];
        let new_val = b0 * val + b1 * hist[2] + b2 * hist[1] + b3 * hist[0];
        hist[0] = hist[1]; hist[1] = hist[2]; hist[2] = new_val;
        line[i] = new_val;
    }
}

fn iir_pass_backward(line: &mut [f64], b1: f64, b2: f64, b3: f64, b0: f64, sigma: f64) {
    let n = line.len();
    if n == 0 { return; }
    
    let pad_len = ((8.0 * sigma).ceil() as usize).max(3);
    let mut hist = [0.0; 3];
    
    for i in (0..(pad_len as isize)).rev() {
        let real_i = (n as isize) + i;
        let idx = reflect_index(real_i, n as isize);
        let val = line[idx];
        let new_val = b0 * val + b1 * hist[2] + b2 * hist[1] + b3 * hist[0];
        hist[0] = hist[1]; hist[1] = hist[2]; hist[2] = new_val;
    }
    
    for i in (0..n).rev() {
        let val = line[i];
        let new_val = b0 * val + b1 * hist[2] + b2 * hist[1] + b3 * hist[0];
        hist[0] = hist[1]; hist[1] = hist[2]; hist[2] = new_val;
        line[i] = new_val;
    }
}

fn iir_gaussian_1ch(buf: &mut Vec<f64>, width: usize, height: usize, sigma: f64) {
    let (b1, b2, b3, b0) = yvv_coeffs(sigma);

    // Horizontal
    for row in 0..height {
        let start = row * width;
        let line = &mut buf[start..start + width];
        iir_pass_forward(line, b1, b2, b3, b0, sigma);
        iir_pass_backward(line, b1, b2, b3, b0, sigma);
    }

    // Vertical — extract column, process, write back
    let mut col_buf = vec![0.0f64; height];
    for col in 0..width {
        for row in 0..height { col_buf[row] = buf[row * width + col]; }
        iir_pass_forward(&mut col_buf, b1, b2, b3, b0, sigma);
        iir_pass_backward(&mut col_buf, b1, b2, b3, b0, sigma);
        for row in 0..height { buf[row * width + col] = col_buf[row]; }
    }
}
