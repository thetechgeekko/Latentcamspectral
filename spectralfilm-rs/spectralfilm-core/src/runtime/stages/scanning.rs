//! Scanning stage: film/print CMY density → output RGB.

use crate::config::{CMFS, N_WL};
use crate::model::diffusion::{apply_gaussian_blur, apply_unsharp_mask};
use crate::model::emulsion::compute_density_spectral;
use crate::model::glare::add_glare;
use crate::model::illuminants::standard_illuminant;
use crate::profiles::Profile;
use crate::runtime::params::{FilmRenderingParams, IOParams, PrintRenderingParams, ScannerParams, SimulationSettings};
use crate::runtime::services::{ColorReferenceService, SpectralLUTService};
use crate::utils::conversions::{density_to_light, light_to_xyz, srgb_gamma_encode, xyz_to_srgb_linear};

#[derive(Debug, Clone)]
pub struct ScanningStage {
    film: Profile,
    film_render: FilmRenderingParams,
    print: Profile,
    print_render: PrintRenderingParams,
    scanner: ScannerParams,
    io: IOParams,
    settings: SimulationSettings,
}

impl ScanningStage {
    pub fn new(
        film: &Profile,
        film_render: &FilmRenderingParams,
        print: &Profile,
        print_render: &PrintRenderingParams,
        scanner: &ScannerParams,
        io: &IOParams,
        settings: &SimulationSettings,
    ) -> Self {
        Self {
            film: film.clone(),
            film_render: film_render.clone(),
            print: print.clone(),
            print_render: print_render.clone(),
            scanner: scanner.clone(),
            io: io.clone(),
            settings: settings.clone(),
        }
    }

    pub fn scan(&self, density_channels: &[f64], width: usize, height: usize, lut_service: &mut SpectralLUTService, color_reference: &ColorReferenceService) -> Vec<f64> {
        let (profile, glare, viewing_illuminant) = if self.io.scan_film {
            (&self.film, None, self.film.info.viewing_illuminant.as_str())
        } else {
            (&self.print, Some(&self.print_render.glare), self.print.info.viewing_illuminant.as_str())
        };
        let n_wl = profile.data.n_wl().min(N_WL);
        let scan_illuminant = standard_illuminant(viewing_illuminant);
        let normalization: f64 = (0..n_wl).map(|wl| scan_illuminant[wl] * CMFS[wl][1]).sum::<f64>().max(1e-12);

        let spectral_calculation = |cmy_data: &[f64]| -> Vec<f64> {
            self.cmy_to_log_xyz(cmy_data, color_reference)
        };

        let density_min = [0.0, 0.0, 0.0];
        let mut density_max = [-f64::INFINITY; 3];
        for (i, v) in profile.data.density_curves.iter().enumerate() {
            if v.is_finite() && *v > density_max[i % 3] {
                density_max[i % 3] = *v;
            }
        }
        for ch in 0..3 {
            if density_max[ch] == -f64::INFINITY { density_max[ch] = 3.0; }
        }

        let log_xyz = lut_service.spectral_compute_scanner(
            density_channels,
            spectral_calculation,
            density_min,
            density_max,
            self.settings.use_scanner_lut,
        );

        let mut xyz = log_xyz.into_iter().map(|v| 10.0_f64.powf(v)).collect::<Vec<_>>();

        if let Some(g) = glare {
            let illuminant_xyz = illuminant_xyz(&scan_illuminant, n_wl, normalization);
            add_glare(&mut xyz, &illuminant_xyz, g, width, height);
        }

        let mut rgb = xyz_to_srgb_linear(&xyz);
        apply_gaussian_blur(&mut rgb, width, height, 3, self.scanner.lens_blur);
        let (sigma, amount) = self.scanner.unsharp_mask;
        if sigma > 0.0 && amount > 0.0 {
            rgb = apply_unsharp_mask(&rgb, width, height, 3, sigma, amount);
        }
        if self.io.output_cctf_encoding {
            rgb = srgb_gamma_encode(&rgb);
        }
        for v in &mut rgb { *v = v.clamp(0.0, 1.0); }
        rgb
    }

    pub fn cmy_to_log_xyz(&self, cmy: &[f64], color_reference: &ColorReferenceService) -> Vec<f64> {
        let profile = if self.io.scan_film { &self.film } else { &self.print };
        let viewing_illuminant = profile.info.viewing_illuminant.as_str();
        let n_wl = profile.data.n_wl().min(N_WL);
        let scan_illuminant = standard_illuminant(viewing_illuminant);
        let normalization: f64 = (0..n_wl).map(|wl| scan_illuminant[wl] * CMFS[wl][1]).sum::<f64>().max(1e-12);

        let density_spectral = compute_density_spectral(
            &profile.data.channel_density,
            cmy,
            &profile.data.base_density,
            n_wl,
        );
        let light = density_to_light(&density_spectral, &scan_illuminant, n_wl);
        let mut xyz = light_to_xyz(&light, &CMFS, normalization, n_wl);
        color_reference.xyz_correction_for_profile(&mut xyz, profile);
        xyz.into_iter().map(|v| (v.max(0.0) + 1e-10).log10()).collect()
    }
}

fn illuminant_xyz(illuminant: &[f64; N_WL], n_wl: usize, normalization: f64) -> [f64; 3] {
    let mut xyz = [0.0; 3];
    for wl in 0..n_wl {
        for ch in 0..3 { xyz[ch] += illuminant[wl] * CMFS[wl][ch]; }
    }
    for v in &mut xyz { *v /= normalization.max(1e-12); }
    xyz
}
