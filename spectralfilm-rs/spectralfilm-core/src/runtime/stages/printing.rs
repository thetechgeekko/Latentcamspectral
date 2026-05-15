//! Printing stage: developed film CMY density → print log raw → print CMY density.

use crate::config::N_WL;
use crate::model::diffusion::apply_diffusion_filter_um;
use crate::model::emulsion::{compute_density_spectral, develop_simple};
use crate::profiles::Profile;
use crate::runtime::params::{EnlargerParams, FilmRenderingParams, PrintRenderingParams, SimulationSettings};
use crate::runtime::services::{ColorReferenceService, EnlargerService, SpectralLUTService};
use crate::utils::conversions::density_to_light;

#[derive(Debug, Clone)]
pub struct PrintingStage {
    film: Profile,
    film_render: FilmRenderingParams,
    print: Profile,
    print_render: PrintRenderingParams,
    enlarger: EnlargerParams,
    settings: SimulationSettings,
}

impl PrintingStage {
    pub fn new(
        film: &Profile,
        film_render: &FilmRenderingParams,
        print: &Profile,
        print_render: &PrintRenderingParams,
        enlarger: &EnlargerParams,
        settings: &SimulationSettings,
    ) -> Self {
        Self {
            film: film.clone(),
            film_render: film_render.clone(),
            print: print.clone(),
            print_render: print_render.clone(),
            enlarger: enlarger.clone(),
            settings: settings.clone(),
        }
    }

    pub fn expose(&self, cmy_film: &[f64], width: usize, height: usize, pixel_size_um: f64, lut_service: &mut SpectralLUTService, enlarger_service: &EnlargerService, color_reference: &mut ColorReferenceService) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let _n_px = width * height;
        let n_wl = self.film.data.n_wl().min(N_WL);
        let print_light = enlarger_service.filtered_illuminant();
        
        let sensitivity: Vec<f64> = self.print.data.log_sensitivity.iter()
            .map(|v| if v.is_finite() { 10.0_f64.powf(*v) } else { 0.0 })
            .collect();

        let mut preflash_components = [0.0; 3];
        if self.enlarger.preflash_exposure > 0.0 {
            let preflash_light = enlarger_service.preflash_illuminant();
            let base_density = &self.film.data.base_density;
            for ch in 0..3 {
                let mut pre = 0.0;
                for wl in 0..n_wl {
                    let bd = base_density.get(wl).copied().unwrap_or(0.0);
                    let t = if bd.is_finite() { 10.0_f64.powf(-bd) } else { 0.0 };
                    let s = sensitivity.get(wl * 3 + ch).copied().unwrap_or(0.0);
                    pre += t * preflash_light[wl] * s;
                }
                preflash_components[ch] = pre * self.enlarger.preflash_exposure;
            }
        }

        let spectral_calculation = |cmy_data: &[f64]| -> Vec<f64> {
            let n_px_closure = cmy_data.len() / 3;
            let density_spectral = compute_density_spectral(
                &self.film.data.channel_density,
                cmy_data,
                &self.film.data.base_density,
                n_wl,
            );
            let transmitted = density_to_light(&density_spectral, &print_light, n_wl);
            let mut raw_closure = vec![0.0; n_px_closure * 3];
            for px in 0..n_px_closure {
                for ch in 0..3 {
                    let mut v = 0.0;
                    for wl in 0..n_wl {
                        let s = sensitivity.get(wl * 3 + ch).copied().unwrap_or(0.0);
                        if s.is_finite() { v += transmitted[px * n_wl + wl] * s; }
                    }
                    raw_closure[px * 3 + ch] = (v + preflash_components[ch]).max(0.0) + 1e-10;
                }
            }
            raw_closure.into_iter().map(|v| v.log10()).collect()
        };

        let data_min = [
            -self.film_render.grain.density_min[0],
            -self.film_render.grain.density_min[1],
            -self.film_render.grain.density_min[2],
        ];
        let mut data_max = [-f64::INFINITY; 3];
        for (i, v) in self.film.data.density_curves.iter().enumerate() {
            if v.is_finite() && *v > data_max[i % 3] {
                data_max[i % 3] = *v;
            }
        }
        for ch in 0..3 {
            if data_max[ch] == -f64::INFINITY { data_max[ch] = 3.0; }
        }

        let log_raw = lut_service.spectral_compute_enlarger(
            cmy_film,
            spectral_calculation,
            data_min,
            data_max,
            self.settings.use_enlarger_lut,
        );

        let cmy_black_white = [
            data_max[0], data_max[1], data_max[2],
            0.0, 0.0, 0.0,
        ];
        let log_raw_print_bw = lut_service.spectral_compute_enlarger(
            &cmy_black_white,
            spectral_calculation,
            data_min,
            data_max,
            self.settings.use_enlarger_lut,
        );

        let exposure_factor = enlarger_service.compute_exposure_factor_midgray(&sensitivity, &print_light);
        let bw_correction = color_reference.black_white_printing_exposure_correction(&self.print);
        let print_exposure_adjusted = self.enlarger.print_exposure * exposure_factor * bw_correction;

        let mut raw = log_raw.into_iter().map(|v| 10.0_f64.powf(v)).collect::<Vec<_>>();
        for v in &mut raw {
            *v *= print_exposure_adjusted;
        }

        raw = apply_diffusion_filter_um(&raw, &self.enlarger.diffusion_filter, pixel_size_um, width, height, 3);
        let final_log_raw = raw.into_iter().map(|v| (v.max(0.0) + 1e-10).log10()).collect();

        let mut raw_bw = log_raw_print_bw.into_iter().map(|v| 10.0_f64.powf(v)).collect::<Vec<_>>();
        for v in &mut raw_bw {
            *v *= print_exposure_adjusted;
        }
        let final_log_raw_bw: Vec<f64> = raw_bw.into_iter().map(|v| (v.max(0.0) + 1e-10).log10()).collect();

        (final_log_raw, final_log_raw_bw[0..3].to_vec(), final_log_raw_bw[3..6].to_vec())
    }

    pub fn develop(&self, log_raw_print: &[f64]) -> Vec<f64> {
        develop_simple(
            log_raw_print,
            &self.print.data.log_exposure,
            &self.print.data.density_curves,
            [self.print_render.density_curve_gamma; 3],
        )
    }
}
