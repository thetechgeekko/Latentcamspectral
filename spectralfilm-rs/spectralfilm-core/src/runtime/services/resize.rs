//! Shared crop/resize operations and physical pixel-size tracking.

use crate::runtime::params::IOParams;
use crate::utils::crop_resize::{crop_image, resize_bilinear, resize_for_preview};

#[derive(Debug, Clone)]
pub struct ResizingService {
    io: IOParams,
    film_format_mm: f64,
    pixel_size_um: f64,
}

impl ResizingService {
    pub fn new(io: &IOParams, film_format_mm: f64) -> Self {
        Self { io: io.clone(), film_format_mm, pixel_size_um: 1.0 }
    }

    pub fn pixel_size_um(&self) -> f64 { self.pixel_size_um }

    pub fn crop_and_rescale(&mut self, image: &[f64], width: usize, height: usize, channels: usize) -> (Vec<f64>, usize, usize) {
        self.pixel_size_um = self.film_format_mm * 1000.0 / width.max(height) as f64;
        let (mut img, mut w, mut h) = (image.to_vec(), width, height);
        if self.io.crop {
            let cropped = crop_image(&img, w, h, channels, self.io.crop_center, self.io.crop_size);
            img = cropped.0;
            w = cropped.1;
            h = cropped.2;
        }
        if (self.io.upscale_factor - 1.0).abs() > f64::EPSILON {
            let scale = self.io.upscale_factor.max(0.01);
            let new_w = ((w as f64 * scale).round() as usize).max(1);
            let new_h = ((h as f64 * scale).round() as usize).max(1);
            self.pixel_size_um /= scale;
            img = resize_bilinear(&img, w, h, new_w, new_h, channels);
            w = new_w;
            h = new_h;
        }
        (img, w, h)
    }

    pub fn small_preview(&self, image: &[f64], width: usize, height: usize, channels: usize, max_size: usize) -> (Vec<f64>, usize, usize) {
        resize_for_preview(image, width, height, channels, max_size)
    }
}
