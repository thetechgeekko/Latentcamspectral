//! Black/white reference correction service.
//!
//! The Python implementation shares state between filming, printing and
//! scanning stages. This Rust port exposes the same pieces of state and the
//! same correction math as service methods so the stages can integrate them
//! without duplicating reference/exposure-balancing logic.

use crate::model::emulsion::develop_simple;
use crate::profiles::Profile;
use crate::runtime::params::{IOParams, PrintRenderingParams, ScannerParams};

const EPS: f64 = 1e-10;
const MIDGRAY_Y: f64 = 0.184;

#[derive(Debug, Clone)]
pub struct ColorReferenceService {
    scan_film: bool,
    black_correction: bool,
    white_correction: bool,
    black_level: f64,
    white_level: f64,

    // Positive film / print reference luminance values in scan XYZ space.
    y_black: Option<f64>,
    y_white: Option<f64>,

    // Exposure correction factor cached by callers that want to avoid
    // recomputing profile interpolation repeatedly.
    black_white_exposure_correction: Option<f64>,

    // Communication with the printing stage: raw print references before
    // paper development, matching Python's log_raw_print_black/white fields.
    log_raw_print_black: Option<Vec<f64>>,
    log_raw_print_white: Option<Vec<f64>>,

    // Optional context for the scanning stage. When set to "negative" for a
    // film scan, correction is bypassed just like Python.
    scan_profile_type: Option<String>,
}

impl ColorReferenceService {
    pub fn new(scanner: &ScannerParams, io: &IOParams) -> Self {
        Self {
            scan_film: io.scan_film,
            black_correction: scanner.black_correction,
            white_correction: scanner.white_correction,
            black_level: remove_srgb_cctf(scanner.black_level),
            white_level: remove_srgb_cctf(scanner.white_level),
            y_black: None,
            y_white: None,
            black_white_exposure_correction: None,
            log_raw_print_black: None,
            log_raw_print_white: None,
            scan_profile_type: None,
        }
    }

    /// Backwards-compatible identity placeholder for stages that have not yet
    /// been wired with profile/reference data.
    pub fn filming_exposure_correction(&self) -> f64 { 1.0 }

    /// Backwards-compatible identity placeholder for stages that have not yet
    /// been wired with profile/reference data.
    pub fn printing_exposure_correction(&self) -> f64 { 1.0 }

    pub fn corrections_enabled(&self) -> bool { self.black_correction || self.white_correction }

    pub fn black_level(&self) -> f64 { self.black_level }

    pub fn white_level(&self) -> f64 { self.white_level }

    pub fn reference_y_black(&self) -> Option<f64> { self.y_black }

    pub fn reference_y_white(&self) -> Option<f64> { self.y_white }

    pub fn cached_exposure_correction(&self) -> Option<f64> { self.black_white_exposure_correction }

    pub fn clear_cached_exposure_correction(&mut self) {
        self.black_white_exposure_correction = None;
    }

    pub fn set_scan_context(&mut self, scan_film: bool, profile_type: impl Into<String>) {
        self.scan_film = scan_film;
        self.scan_profile_type = Some(profile_type.into());
    }

    pub fn set_reference_y(&mut self, y_black: f64, y_white: f64) {
        self.y_black = finite_or_none(y_black);
        self.y_white = finite_or_none(y_white);
        self.black_white_exposure_correction = None;
    }

    pub fn clear_reference_y(&mut self) {
        self.y_black = None;
        self.y_white = None;
        self.black_white_exposure_correction = None;
    }

    /// Store reference Y values from log10 XYZ triples. The Y component from
    /// the first pixel is used, matching the 1×1 Python references.
    pub fn set_reference_y_from_log_xyz(&mut self, log_xyz_black: &[f64], log_xyz_white: &[f64]) -> bool {
        let Some(y_black) = y_from_log_xyz(log_xyz_black) else { return false; };
        let Some(y_white) = y_from_log_xyz(log_xyz_white) else { return false; };
        self.set_reference_y(y_black, y_white);
        true
    }

    pub fn set_log_raw_print_references(&mut self, black: Vec<f64>, white: Vec<f64>) {
        self.log_raw_print_black = Some(black);
        self.log_raw_print_white = Some(white);
        self.black_white_exposure_correction = None;
    }

    pub fn set_log_raw_print_black(&mut self, black: Vec<f64>) {
        self.log_raw_print_black = Some(black);
        self.black_white_exposure_correction = None;
    }

    pub fn set_log_raw_print_white(&mut self, white: Vec<f64>) {
        self.log_raw_print_white = Some(white);
        self.black_white_exposure_correction = None;
    }

    pub fn log_raw_print_black(&self) -> Option<&[f64]> { self.log_raw_print_black.as_deref() }

    pub fn log_raw_print_white(&self) -> Option<&[f64]> { self.log_raw_print_white.as_deref() }

    pub fn clear_log_raw_print_references(&mut self) {
        self.log_raw_print_black = None;
        self.log_raw_print_white = None;
        self.black_white_exposure_correction = None;
    }

    /// Compute positive-film black/white references using the scanner's
    /// CMY→logXYZ callable. This mirrors Python's positive film branch:
    /// black = max film density curves, white = zero CMY density.
    pub fn update_positive_film_references<F>(&mut self, film: &Profile, mut cmy_to_log_xyz: F) -> bool
    where
        F: FnMut(&[f64]) -> Vec<f64>,
    {
        if !self.corrections_enabled() || !(self.scan_film && film.is_positive()) {
            return false;
        }
        let cmy_black = channel_nanmax(&film.data.density_curves, film.data.n_le());
        let cmy_white = [0.0, 0.0, 0.0];
        let log_xyz_black = cmy_to_log_xyz(&cmy_black);
        let log_xyz_white = cmy_to_log_xyz(&cmy_white);
        self.set_reference_y_from_log_xyz(&log_xyz_black, &log_xyz_white)
    }

    /// Compute negative-print black/white references from stored raw print
    /// reference exposures, paper development curves, and scanner CMY→logXYZ.
    pub fn update_negative_print_references<F>(
        &mut self,
        print: &Profile,
        print_render: &PrintRenderingParams,
        mut cmy_to_log_xyz: F,
    ) -> bool
    where
        F: FnMut(&[f64]) -> Vec<f64>,
    {
        if !self.corrections_enabled() || !print.is_negative() {
            return false;
        }
        let (Some(log_black), Some(log_white)) = (self.log_raw_print_black.as_ref(), self.log_raw_print_white.as_ref()) else {
            return false;
        };
        let gamma = [print_render.density_curve_gamma; 3];
        let cmy_black = develop_simple(log_black, &print.data.log_exposure, &print.data.density_curves, gamma);
        let cmy_white = develop_simple(log_white, &print.data.log_exposure, &print.data.density_curves, gamma);
        let log_xyz_black = cmy_to_log_xyz(&cmy_black);
        let log_xyz_white = cmy_to_log_xyz(&cmy_white);
        self.set_reference_y_from_log_xyz(&log_xyz_black, &log_xyz_white)
    }

    /// Python-equivalent `black_white_filming_exposure_correction` once the
    /// positive-film Y references have been populated.
    pub fn black_white_filming_exposure_correction(&mut self, film: &Profile) -> f64 {
        if !self.corrections_enabled() || film.is_negative() || !(self.scan_film && film.is_positive()) {
            return 1.0;
        }
        let Some((m, q)) = self.correction_coefficients() else { return 1.0; };
        let Some(midgray_corrected) = corrected_midgray_input(m, q) else { return 1.0; };

        let density_midgray = -MIDGRAY_Y.log10();
        let density_midgray_corrected = -midgray_corrected.log10();
        let curve_av = density_curve_channel_mean(&film.data.density_curves, film.data.n_le());
        let density_min_av = nanmean(&film.data.base_density).unwrap_or(0.0);

        let x_corr = -(density_midgray_corrected - density_min_av);
        let x_mid = -(density_midgray - density_min_av);
        let axis: Vec<f64> = curve_av.iter().map(|v| -*v).collect();
        let le_corr = -interp_clamped_monotonic(x_corr, &axis, &film.data.log_exposure);
        let le_mid = -interp_clamped_monotonic(x_mid, &axis, &film.data.log_exposure);
        let exposure_correction = 10.0_f64.powf(le_corr - le_mid);
        let out = if exposure_correction.is_finite() && exposure_correction.abs() > EPS {
            1.0 / exposure_correction
        } else {
            1.0
        };
        self.black_white_exposure_correction = Some(out);
        out
    }

    /// Python-equivalent `black_white_printing_exposure_correction` once the
    /// negative-print Y references have been populated.
    pub fn black_white_printing_exposure_correction(&mut self, print: &Profile) -> f64 {
        if !self.corrections_enabled() || !print.is_negative() {
            return 1.0;
        }
        let Some((m, q)) = self.correction_coefficients() else { return 1.0; };
        let Some(midgray_corrected) = corrected_midgray_input(m, q) else { return 1.0; };

        let density_midgray = -MIDGRAY_Y.log10();
        let density_midgray_corrected = -midgray_corrected.log10();
        let curve_av = density_curve_channel_mean(&print.data.density_curves, print.data.n_le());
        let density_min_av = nanmean(&print.data.base_density).unwrap_or(0.0);

        let le_corr = interp_clamped_monotonic(
            density_midgray_corrected - density_min_av,
            &curve_av,
            &print.data.log_exposure,
        );
        let le_mid = interp_clamped_monotonic(
            density_midgray - density_min_av,
            &curve_av,
            &print.data.log_exposure,
        );
        let out = 10.0_f64.powf(le_corr - le_mid);
        let out = if out.is_finite() { out } else { 1.0 };
        self.black_white_exposure_correction = Some(out);
        out
    }

    /// Apply black/white correction to XYZ values using precomputed reference
    /// Y values. If references are missing, this intentionally does nothing;
    /// unlike the previous Rust fallback, Python never derives references from
    /// image min/max during scanning.
    pub fn xyz_correction(&self, xyz: &mut [f64]) {
        if !self.corrections_enabled() || self.should_skip_negative_film_scan() {
            return;
        }
        let Some((m, q)) = self.correction_coefficients() else { return; };
        apply_xyz_y_correction(xyz, m, q);
    }

    /// Profile-aware variant useful for stage integration without first
    /// calling `set_scan_context`.
    pub fn xyz_correction_for_profile(&self, xyz: &mut [f64], profile: &Profile) {
        if self.scan_film && profile.is_negative() {
            return;
        }
        self.xyz_correction(xyz);
    }

    fn should_skip_negative_film_scan(&self) -> bool {
        self.scan_film && self.scan_profile_type.as_deref() == Some("negative")
    }

    fn correction_coefficients(&self) -> Option<(f64, f64)> {
        if !self.corrections_enabled() {
            return None;
        }
        let y_black = self.y_black?;
        let y_white = self.y_white?;
        let black_level = if self.black_correction { self.black_level } else { y_black };
        let white_level = if self.white_correction { self.white_level } else { y_white };
        let denom = y_white - y_black + EPS;
        let m = (white_level - black_level) / denom;
        let q = black_level - m * y_black;
        if m.is_finite() && q.is_finite() && m.abs() > EPS { Some((m, q)) } else { None }
    }
}

fn apply_xyz_y_correction(xyz: &mut [f64], m: f64, q: f64) {
    let n_px = xyz.len() / 3;
    for px in 0..n_px {
        let y = xyz[px * 3 + 1];
        if !y.is_finite() {
            continue;
        }
        let y_new = (m * y + q).clamp(0.0, 1.0);
        let scale = y_new / (y + EPS);
        xyz[px * 3] *= scale;
        xyz[px * 3 + 1] = y_new;
        xyz[px * 3 + 2] *= scale;
    }
}

fn corrected_midgray_input(m: f64, q: f64) -> Option<f64> {
    let v = (MIDGRAY_Y - q) / m;
    if v.is_finite() && v > EPS { Some(v) } else { None }
}

fn y_from_log_xyz(log_xyz: &[f64]) -> Option<f64> {
    let log_y = *log_xyz.get(1)?;
    if !log_y.is_finite() {
        return None;
    }
    finite_or_none(10.0_f64.powf(log_y))
}

fn finite_or_none(v: f64) -> Option<f64> {
    if v.is_finite() { Some(v) } else { None }
}

fn channel_nanmax(values: &[f64], rows: usize) -> [f64; 3] {
    let mut out = [f64::NEG_INFINITY; 3];
    for row in 0..rows {
        for ch in 0..3 {
            let v = values.get(row * 3 + ch).copied().unwrap_or(f64::NAN);
            if v.is_finite() && v > out[ch] {
                out[ch] = v;
            }
        }
    }
    for v in &mut out {
        if !v.is_finite() {
            *v = 0.0;
        }
    }
    out
}

fn density_curve_channel_mean(values: &[f64], rows: usize) -> Vec<f64> {
    let mut out = vec![0.0; rows];
    for row in 0..rows {
        let mut sum = 0.0;
        let mut count = 0usize;
        for ch in 0..3 {
            let v = values.get(row * 3 + ch).copied().unwrap_or(f64::NAN);
            if v.is_finite() {
                sum += v;
                count += 1;
            }
        }
        out[row] = if count > 0 { sum / count as f64 } else { 0.0 };
    }
    out
}

fn nanmean(values: &[f64]) -> Option<f64> {
    let mut sum = 0.0;
    let mut count = 0usize;
    for &v in values {
        if v.is_finite() {
            sum += v;
            count += 1;
        }
    }
    if count > 0 { Some(sum / count as f64) } else { None }
}

fn interp_clamped_monotonic(x: f64, xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len().min(ys.len());
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return ys[0];
    }

    let increasing = xs[n - 1] >= xs[0];
    if increasing {
        if x <= xs[0] { return ys[0]; }
        if x >= xs[n - 1] { return ys[n - 1]; }
        for i in 0..(n - 1) {
            if x >= xs[i] && x <= xs[i + 1] {
                return lerp_on_segment(x, xs[i], xs[i + 1], ys[i], ys[i + 1]);
            }
        }
    } else {
        if x >= xs[0] { return ys[0]; }
        if x <= xs[n - 1] { return ys[n - 1]; }
        for i in 0..(n - 1) {
            if x <= xs[i] && x >= xs[i + 1] {
                return lerp_on_segment(x, xs[i], xs[i + 1], ys[i], ys[i + 1]);
            }
        }
    }
    ys[n - 1]
}

fn lerp_on_segment(x: f64, x0: f64, x1: f64, y0: f64, y1: f64) -> f64 {
    let dx = x1 - x0;
    if dx.abs() < 1e-30 {
        y0
    } else {
        let t = (x - x0) / dx;
        y0 + t * (y1 - y0)
    }
}

fn remove_srgb_cctf(y: f64) -> f64 {
    if y <= 0.04045 { y / 12.92 } else { ((y + 0.055) / 1.055).powf(2.4) }
}
