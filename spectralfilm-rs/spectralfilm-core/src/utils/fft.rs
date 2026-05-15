use rustfft::num_complex::Complex64;
use rustfft::FftPlanner;

/// Performs a 2D FFT convolution of the `image` and `psf`.
/// Both `image` and `psf` must have length `width * height`.
/// The `psf` is assumed to be centered at `(width/2, height/2)`.
/// Returns the convolved image of the same size.
pub fn convolve_2d_fft(
    image: &[f64],
    psf: &[f64],
    width: usize,
    height: usize,
) -> Vec<f64> {
    let n = width * height;
    if n == 0 {
        return Vec::new();
    }

    let mut planner = FftPlanner::<f64>::new();
    let fft_row = planner.plan_fft_forward(width);
    let fft_col = planner.plan_fft_forward(height);
    let ifft_row = planner.plan_fft_inverse(width);
    let ifft_col = planner.plan_fft_inverse(height);

    // Prepare complex buffers
    let mut img_complex = vec![Complex64::new(0.0, 0.0); n];
    let mut psf_complex = vec![Complex64::new(0.0, 0.0); n];

    for i in 0..n {
        img_complex[i].re = image[i];
        psf_complex[i].re = psf[i];
    }

    // 2D Forward FFT on Image
    fft2d(&mut img_complex, width, height, &*fft_row, &*fft_col);

    // 2D Forward FFT on PSF
    fft2d(&mut psf_complex, width, height, &*fft_row, &*fft_col);

    // Multiply in frequency domain
    for i in 0..n {
        img_complex[i] = img_complex[i] * psf_complex[i];
    }

    // 2D Inverse FFT
    ifft2d(&mut img_complex, width, height, &*ifft_row, &*ifft_col);

    // Extract real part, normalize by (W * H), and handle the PSF shift.
    // The PSF was centered at (width/2, height/2). 
    // Circular convolution with a shifted PSF shifts the result by the same amount.
    let mut out = vec![0.0; n];
    let scale = 1.0 / (n as f64);
    
    let shift_x = width / 2;
    let shift_y = height / 2;

    for y in 0..height {
        let src_y = (y + shift_y) % height;
        for x in 0..width {
            let src_x = (x + shift_x) % width;
            let src_idx = src_y * width + src_x;
            let dst_idx = y * width + x;
            out[dst_idx] = img_complex[src_idx].re * scale;
        }
    }

    out
}

fn fft2d(
    data: &mut [Complex64],
    width: usize,
    height: usize,
    fft_row: &dyn rustfft::Fft<f64>,
    fft_col: &dyn rustfft::Fft<f64>,
) {
    // Row passes
    for y in 0..height {
        let row = &mut data[y * width..(y + 1) * width];
        fft_row.process(row);
    }

    // Column passes (transpose -> process -> transpose)
    // For small/medium sizes, out-of-place transpose is fast enough
    let mut col_buffer = vec![Complex64::new(0.0, 0.0); width * height];
    
    for y in 0..height {
        for x in 0..width {
            col_buffer[x * height + y] = data[y * width + x];
        }
    }

    for x in 0..width {
        let col = &mut col_buffer[x * height..(x + 1) * height];
        fft_col.process(col);
    }

    for x in 0..width {
        for y in 0..height {
            data[y * width + x] = col_buffer[x * height + y];
        }
    }
}

fn ifft2d(
    data: &mut [Complex64],
    width: usize,
    height: usize,
    ifft_row: &dyn rustfft::Fft<f64>,
    ifft_col: &dyn rustfft::Fft<f64>,
) {
    // Same structure as forward 2D FFT
    for y in 0..height {
        let row = &mut data[y * width..(y + 1) * width];
        ifft_row.process(row);
    }

    let mut col_buffer = vec![Complex64::new(0.0, 0.0); width * height];
    
    for y in 0..height {
        for x in 0..width {
            col_buffer[x * height + y] = data[y * width + x];
        }
    }

    for x in 0..width {
        let col = &mut col_buffer[x * height..(x + 1) * height];
        ifft_col.process(col);
    }

    for x in 0..width {
        for y in 0..height {
            data[y * width + x] = col_buffer[x * height + y];
        }
    }
}
