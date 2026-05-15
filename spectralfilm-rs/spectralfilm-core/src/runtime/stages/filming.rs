//! Filming stage: RGB input → log raw exposure → developed film CMY density.

use crate::model::color_filters::compute_band_pass_filter;
use crate::model::diffusion::{apply_diffusion_filter_um, apply_gaussian_blur_um, apply_halation_um, boost_highlights};
use crate::model::emulsion::{compute_density_spectral, develop, DevelopParams};
use crate::profiles::Profile;
use crate::runtime::params::{CameraParams, FilmRenderingParams, IOParams, SimulationSettings};
use crate::runtime::services::SpectralLUTService;
use crate::utils::autoexposure::measure_autoexposure_ev;
use crate::utils::spectral_upsampling::{compute_hanatos2025_tc_lut, rgb_to_raw_hanatos2025, rgb_to_raw_mallett2019};

#[derive(Debug, Clone)]
pub struct FilmingStage {
    film: Profile,
    film_render: FilmRenderingParams,
    camera: CameraParams,
    io: IOParams,
    settings: SimulationSettings,
}

impl FilmingStage {
    pub fn new(film: &Profile, film_render: &FilmRenderingParams, camera: &CameraParams, io: &IOParams, settings: &SimulationSettings) -> Self {
        Self { film: film.clone(), film_render: film_render.clone(), camera: camera.clone(), io: io.clone(), settings: settings.clone() }
    }

    pub fn auto_exposure(&self, image: &mut [f64], width: usize, height: usize) {
        if !self.camera.auto_exposure { return; }
        let ev = measure_autoexposure_ev(image, width, height, &self.camera.auto_exposure_method);
        let scale = 2.0_f64.powf(ev);
        for v in image { *v *= scale; }
    }

    pub fn expose(&self, image: &[f64], width: usize, height: usize, pixel_size_um: f64, lut_service: &mut SpectralLUTService) -> Vec<f64> {
        let mut sensitivity: Vec<f64> = self.film.data.log_sensitivity.iter()
            .map(|v| if v.is_finite() { 10.0_f64.powf(*v) } else { 0.0 })
            .collect();

        if self.camera.filter_uv.0 > 0.0 || self.camera.filter_ir.0 > 0.0 {
            let bp = compute_band_pass_filter(self.camera.filter_uv, self.camera.filter_ir);
            for wl in 0..self.film.data.n_wl().min(bp.len()) {
                for ch in 0..3 { sensitivity[wl * 3 + ch] *= bp[wl]; }
            }
        }
        if self.settings.bandpass_hanatos2025 && self.settings.rgb_to_raw_method == "hanatos2025" && self.film.data.bandpass_hanatos2025.len() == sensitivity.len() {
            for (s, bp) in sensitivity.iter_mut().zip(self.film.data.bandpass_hanatos2025.iter()) {
                if bp.is_finite() { *s *= *bp; }
            }
        }

        let mut raw = if self.settings.rgb_to_raw_method == "mallett2019" {
            rgb_to_raw_mallett2019(image, &sensitivity, &self.io.input_color_space, self.io.input_cctf_decoding, &self.film.info.reference_illuminant)
        } else {
            let tc_lut = lut_service.get_filming_tc_lut(&sensitivity, |s| compute_hanatos2025_tc_lut(s));
            rgb_to_raw_hanatos2025(image, &sensitivity, &self.io.input_color_space, self.io.input_cctf_decoding, &self.film.info.reference_illuminant, tc_lut)
        };

        let ev_scale = 2.0_f64.powf(self.camera.exposure_compensation_ev);
        for v in &mut raw { *v *= ev_scale; }
        boost_highlights(&mut raw, self.film_render.halation.boost_ev, self.film_render.halation.boost_range, self.film_render.halation.protect_ev);
        raw = apply_diffusion_filter_um(&raw, &self.camera.diffusion_filter, pixel_size_um, width, height, 3);
        apply_gaussian_blur_um(&mut raw, width, height, 3, self.camera.lens_blur_um, pixel_size_um);
        raw = apply_halation_um(&raw, &self.film_render.halation, pixel_size_um, width, height);

        raw.into_iter().map(|v| (v.max(0.0) + 1e-10).log10()).collect()
    }

    pub fn develop(&self, log_raw: &[f64], width: usize, height: usize, pixel_size_um: f64) -> Vec<f64> {
        let p = DevelopParams {
            log_exposure: &self.film.data.log_exposure,
            density_curves: &self.film.data.density_curves,
            density_curves_layers: &self.film.data.density_curves_layers,
            dir_couplers: &self.film_render.dir_couplers,
            grain: &self.film_render.grain,
            profile_type: &self.film.info.r#type,
            gamma_factor: self.film_render.density_curve_gamma,
            pixel_size_um,
        };
        develop(log_raw, &p, (height, width))
    }

    pub fn compute_midgray_spectral_density(&self, exposure_factor: f64) -> Vec<f64> {
        let midgray = exposure_factor * 0.184;
        let midgray_log = (midgray.max(0.0) + 1e-10).log10();
        let log_raw = vec![midgray_log; 3];
        let cmy = self.develop(&log_raw, 1, 1, 1.0);
        let n_wl = self.film.data.n_wl().min(crate::config::N_WL);
        compute_density_spectral(&self.film.data.channel_density, &cmy, &self.film.data.base_density, n_wl)
    }
}
