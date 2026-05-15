//! Enlarger light-source, CMY filter, and print balancing service.

use crate::config::N_WL;
use crate::model::color_filters::color_enlarger;
use crate::model::illuminants::standard_illuminant;
use crate::runtime::params::EnlargerParams;
use crate::utils::conversions::density_to_light;

const EPS: f64 = 1e-10;

#[derive(Debug, Clone)]
pub struct EnlargerService {
    params: EnlargerParams,
    /// Spectral density of 18.4% scene midgray on the negative, computed by
    /// the filming stage and consumed by print exposure balancing.
    pub density_spectral_midgray: Option<Vec<f64>>,
    /// Optional compensated variant of `density_spectral_midgray`, computed
    /// with camera exposure compensation applied when print exposure
    /// compensation is enabled.
    pub density_spectral_midgray_comp: Option<Vec<f64>>,
}

impl EnlargerService {
    pub fn new(params: &EnlargerParams) -> Self {
        Self {
            params: params.clone(),
            density_spectral_midgray: None,
            density_spectral_midgray_comp: None,
        }
    }

    pub fn update(&mut self, params: &EnlargerParams) {
        self.params = params.clone();
    }

    pub fn params(&self) -> &EnlargerParams { &self.params }

    pub fn print_exposure_compensation(&self) -> bool { self.params.print_exposure_compensation }

    pub fn normalize_print_exposure(&self) -> bool { self.params.normalize_print_exposure }

    pub fn set_density_spectral_midgray(&mut self, density_spectral_midgray: Vec<f64>) {
        self.density_spectral_midgray = Some(density_spectral_midgray);
    }

    pub fn set_density_spectral_midgray_comp(&mut self, density_spectral_midgray_comp: Option<Vec<f64>>) {
        self.density_spectral_midgray_comp = density_spectral_midgray_comp;
    }

    pub fn set_density_spectral_midgray_pair(
        &mut self,
        density_spectral_midgray: Vec<f64>,
        density_spectral_midgray_comp: Option<Vec<f64>>,
    ) {
        self.density_spectral_midgray = Some(density_spectral_midgray);
        self.density_spectral_midgray_comp = density_spectral_midgray_comp;
    }

    pub fn clear_density_spectral_midgray(&mut self) {
        self.density_spectral_midgray = None;
        self.density_spectral_midgray_comp = None;
    }

    pub fn density_spectral_midgray(&self) -> Option<&[f64]> {
        self.density_spectral_midgray.as_deref()
    }

    pub fn density_spectral_midgray_comp(&self) -> Option<&[f64]> {
        self.density_spectral_midgray_comp.as_deref()
    }

    pub fn base_illuminant(&self) -> [f64; N_WL] {
        standard_illuminant(&self.params.illuminant)
    }

    pub fn filtered_illuminant(&self) -> [f64; N_WL] {
        let light = self.base_illuminant();
        let filters = [
            self.params.c_filter_neutral,
            self.params.m_filter_neutral + self.params.m_filter_shift,
            self.params.y_filter_neutral + self.params.y_filter_shift,
        ];
        color_enlarger(&light, &filters)
    }

    pub fn neutral_illuminant(&self) -> [f64; N_WL] {
        let light = self.base_illuminant();
        let filters = [
            self.params.c_filter_neutral,
            self.params.m_filter_neutral,
            self.params.y_filter_neutral,
        ];
        color_enlarger(&light, &filters)
    }

    pub fn preflash_illuminant(&self) -> [f64; N_WL] {
        let light = self.base_illuminant();
        let filters = [
            self.params.c_filter_neutral,
            self.params.m_filter_neutral + self.params.preflash_m_filter_shift,
            self.params.y_filter_neutral + self.params.preflash_y_filter_shift,
        ];
        color_enlarger(&light, &filters)
    }

    /// Python-equivalent `_compute_exposure_factor_midgray` service logic.
    ///
    /// `sensitivity` is the print paper sensitivity table flattened as
    /// `[N_WL × 3]`; `print_illuminant` is usually `filtered_illuminant()`.
    /// Returns `1.0` until the filming stage stores a midgray density.
    pub fn compute_exposure_factor_midgray(&self, sensitivity: &[f64], print_illuminant: &[f64; N_WL]) -> f64 {
        let factor_midgray = self
            .density_spectral_midgray
            .as_deref()
            .map(|density| exposure_factor(sensitivity, print_illuminant, density))
            .unwrap_or(1.0);

        let factor_midgray_comp = self
            .density_spectral_midgray_comp
            .as_deref()
            .map(|density| exposure_factor(sensitivity, print_illuminant, density))
            .unwrap_or(1.0);

        match (self.params.normalize_print_exposure, self.params.print_exposure_compensation) {
            (false, true) => factor_midgray_comp / factor_midgray.max(EPS),
            (true, true) => factor_midgray_comp,
            (true, false) => factor_midgray,
            (false, false) => 1.0,
        }
    }

    pub fn exposure_factor_for_density(&self, sensitivity: &[f64], print_illuminant: &[f64; N_WL], density_spectral: &[f64]) -> f64 {
        exposure_factor(sensitivity, print_illuminant, density_spectral)
    }
}

/// Match Python's `_exposure_factor`: transmit the midgray spectral density
/// through the print illuminant, integrate against paper sensitivities, then
/// normalize by the reciprocal geometric mean over the three paper channels.
pub fn exposure_factor(sensitivity: &[f64], print_illuminant: &[f64; N_WL], density_spectral_midgray: &[f64]) -> f64 {
    let n_wl = density_spectral_midgray.len().min(N_WL);
    if n_wl == 0 {
        return 1.0;
    }

    let light_midgray = density_to_light(density_spectral_midgray, print_illuminant, n_wl);
    let mut raw_midgray = [0.0f64; 3];
    for ch in 0..3 {
        let mut raw = 0.0;
        for wl in 0..n_wl {
            let s = sensitivity.get(wl * 3 + ch).copied().unwrap_or(0.0);
            if s.is_finite() {
                raw += light_midgray[wl] * s;
            }
        }
        raw_midgray[ch] = raw.max(EPS);
    }

    let raw_midgray_geomean = (raw_midgray.iter().map(|v| v.ln()).sum::<f64>() / 3.0).exp();
    if raw_midgray_geomean.is_finite() && raw_midgray_geomean > EPS {
        1.0 / raw_midgray_geomean
    } else {
        1.0
    }
}
