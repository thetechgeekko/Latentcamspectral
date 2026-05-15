//! Image crop and resize utilities.

/// Crop a flat `[height × width × channels]` image.
pub fn crop_image(
    image: &[f64],
    width: usize,
    height: usize,
    channels: usize,
    center: (f64, f64),
    size: (f64, f64),
) -> (Vec<f64>, usize, usize) {
    let long_side = width.max(height) as f64;
    let crop_w = ((size.0 * long_side).round() as usize).clamp(1, width);
    let crop_h = ((size.1 * long_side).round() as usize).clamp(1, height);
    let cx = (center.0.clamp(0.0, 1.0) * width as f64).round() as isize;
    let cy = (center.1.clamp(0.0, 1.0) * height as f64).round() as isize;
    let x0 = (cx - crop_w as isize / 2).clamp(0, (width - crop_w) as isize) as usize;
    let y0 = (cy - crop_h as isize / 2).clamp(0, (height - crop_h) as isize) as usize;

    let mut out = vec![0.0; crop_w * crop_h * channels];
    for y in 0..crop_h {
        for x in 0..crop_w {
            let src = ((y0 + y) * width + (x0 + x)) * channels;
            let dst = (y * crop_w + x) * channels;
            out[dst..dst + channels].copy_from_slice(&image[src..src + channels]);
        }
    }
    (out, crop_w, crop_h)
}

/// Bilinear resize for flat interleaved images.
pub fn resize_bilinear(
    image: &[f64],
    src_w: usize,
    src_h: usize,
    dst_w: usize,
    dst_h: usize,
    channels: usize,
) -> Vec<f64> {
    if src_w == dst_w && src_h == dst_h {
        return image.to_vec();
    }
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return Vec::new();
    }
    let mut out = vec![0.0; dst_w * dst_h * channels];
    for dy in 0..dst_h {
        let sy = if dst_h > 1 { dy as f64 * (src_h - 1) as f64 / (dst_h - 1) as f64 } else { 0.0 };
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(src_h - 1);
        let ty = sy - y0 as f64;
        for dx in 0..dst_w {
            let sx = if dst_w > 1 { dx as f64 * (src_w - 1) as f64 / (dst_w - 1) as f64 } else { 0.0 };
            let x0 = sx.floor() as usize;
            let x1 = (x0 + 1).min(src_w - 1);
            let tx = sx - x0 as f64;
            for c in 0..channels {
                let p00 = image[(y0 * src_w + x0) * channels + c];
                let p01 = image[(y0 * src_w + x1) * channels + c];
                let p10 = image[(y1 * src_w + x0) * channels + c];
                let p11 = image[(y1 * src_w + x1) * channels + c];
                let v0 = p00 * (1.0 - tx) + p01 * tx;
                let v1 = p10 * (1.0 - tx) + p11 * tx;
                out[(dy * dst_w + dx) * channels + c] = v0 * (1.0 - ty) + v1 * ty;
            }
        }
    }
    out
}

/// Bilinear resize for flat interleaved `f32` images.
pub fn resize_bilinear_f32(
    image: &[f32],
    src_w: usize,
    src_h: usize,
    dst_w: usize,
    dst_h: usize,
    channels: usize,
) -> Vec<f32> {
    if src_w == dst_w && src_h == dst_h {
        return image.to_vec();
    }
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 || channels == 0 {
        return Vec::new();
    }
    let mut out = vec![0.0; dst_w * dst_h * channels];
    for dy in 0..dst_h {
        let sy = if dst_h > 1 { dy as f32 * (src_h - 1) as f32 / (dst_h - 1) as f32 } else { 0.0 };
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(src_h - 1);
        let ty = sy - y0 as f32;
        for dx in 0..dst_w {
            let sx = if dst_w > 1 { dx as f32 * (src_w - 1) as f32 / (dst_w - 1) as f32 } else { 0.0 };
            let x0 = sx.floor() as usize;
            let x1 = (x0 + 1).min(src_w - 1);
            let tx = sx - x0 as f32;
            for c in 0..channels {
                let p00 = image[(y0 * src_w + x0) * channels + c];
                let p01 = image[(y0 * src_w + x1) * channels + c];
                let p10 = image[(y1 * src_w + x0) * channels + c];
                let p11 = image[(y1 * src_w + x1) * channels + c];
                let v0 = p00 * (1.0 - tx) + p01 * tx;
                let v1 = p10 * (1.0 - tx) + p11 * tx;
                out[(dy * dst_w + dx) * channels + c] = v0 * (1.0 - ty) + v1 * ty;
            }
        }
    }
    out
}

/// Resize preserving aspect ratio so the longest dimension is at most `max_size`.
pub fn resize_for_preview(
    image: &[f64],
    width: usize,
    height: usize,
    channels: usize,
    max_size: usize,
) -> (Vec<f64>, usize, usize) {
    let long_side = width.max(height);
    if long_side <= max_size || max_size == 0 {
        return (image.to_vec(), width, height);
    }
    let scale = max_size as f64 / long_side as f64;
    let dst_w = ((width as f64 * scale).round() as usize).max(1);
    let dst_h = ((height as f64 * scale).round() as usize).max(1);
    (resize_bilinear(image, width, height, dst_w, dst_h, channels), dst_w, dst_h)
}

/// Resize an `f32` image preserving aspect ratio so the longest dimension is at most `max_size`.
///
/// This mirrors the Python preview helper without depending on desktop image
/// libraries. It is intended for already-decoded app-side buffers.
pub fn resize_for_preview_f32(
    image: &[f32],
    width: usize,
    height: usize,
    channels: usize,
    max_size: usize,
) -> (Vec<f32>, usize, usize) {
    let long_side = width.max(height);
    if long_side <= max_size || max_size == 0 {
        return (image.to_vec(), width, height);
    }
    let scale = max_size as f32 / long_side as f32;
    let dst_w = ((width as f32 * scale).round() as usize).max(1);
    let dst_h = ((height as f32 * scale).round() as usize).max(1);
    (resize_bilinear_f32(image, width, height, dst_w, dst_h, channels), dst_w, dst_h)
}
