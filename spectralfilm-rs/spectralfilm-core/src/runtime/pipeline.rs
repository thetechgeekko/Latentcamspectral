//! High-level simulation pipeline orchestrator.

use std::time::Instant;

use crate::runtime::params::{DebugMode, RuntimePhotoParams};
use crate::runtime::services::{ColorReferenceService, EnlargerService, ResizingService, SpectralLUTService};
use crate::runtime::stages::{FilmingStage, PrintingStage, ScanningStage};
use crate::utils::timings::Timings;

#[derive(Debug, Clone)]
pub struct SimulationPipeline {
    params: RuntimePhotoParams,
    resize: ResizingService,
    enlarger_service: EnlargerService,
    color_reference: ColorReferenceService,
    lut_service: SpectralLUTService,
    filming: FilmingStage,
    printing: PrintingStage,
    scanning: ScanningStage,
    timings: Timings,
    last_elapsed_time: Option<f64>,
}

impl SimulationPipeline {
    pub fn new(mut params: RuntimePhotoParams) -> Self {
        params.apply_database_neutral_print_filters();
        let resize = ResizingService::new(&params.io, params.camera.film_format_mm);
        let enlarger_service = EnlargerService::new(&params.enlarger);
        let color_reference = ColorReferenceService::new(&params.scanner, &params.io);
        let lut_service = SpectralLUTService::new(params.settings.lut_resolution);
        let filming = FilmingStage::new(&params.film, &params.film_render, &params.camera, &params.io, &params.settings);
        let printing = PrintingStage::new(
            &params.film,
            &params.film_render,
            &params.print,
            &params.print_render,
            &params.enlarger,
            &params.settings,
        );
        let scanning = ScanningStage::new(
            &params.film,
            &params.film_render,
            &params.print,
            &params.print_render,
            &params.scanner,
            &params.io,
            &params.settings,
        );
        Self { params, resize, enlarger_service, color_reference, lut_service, filming, printing, scanning, timings: Timings::new(), last_elapsed_time: None }
    }

    pub fn update(&mut self, mut params: RuntimePhotoParams) {
        params.apply_database_neutral_print_filters();
        *self = Self::new(params);
    }

    pub fn process(&mut self, image: &[f64], width: usize, height: usize) -> Result<SimulationOutput, SimulationError> {
        if image.len() < width * height * 3 {
            return Err(SimulationError::InvalidInput("input buffer shorter than width*height*3".into()));
        }
        self.timings.clear();
        let total_start = Instant::now();
        
        self.color_reference.set_scan_context(self.params.io.scan_film, &self.params.film.info.r#type);
        let color_ref_clone = self.color_reference.clone();
        self.color_reference.update_positive_film_references(&self.params.film, |cmy| self.scanning.cmy_to_log_xyz(cmy, &color_ref_clone));

        let mut input = image[..width * height * 3].to_vec();
        
        let (cmy_film, w, h, pixel_size_um) = if self.params.debug.debug_mode == DebugMode::Inject && self.params.debug.inject_film_density_cmy {
            let pixel_size = self.params.camera.film_format_mm * 1000.0 / width.max(height) as f64;
            (input, width, height, pixel_size)
        } else if self.params.debug.debug_mode == DebugMode::LutGeneration {
            self.filming.auto_exposure(&mut input, width, height);
            let px_size = self.resize.pixel_size_um(); // dummy
            let log_raw_film = self.filming.expose(&input, width, height, px_size, &mut self.lut_service);
            let cmy = self.filming.develop(&log_raw_film, width, height, px_size);
            (cmy, width, height, px_size)
        } else {
            let t0 = Instant::now();
            self.filming.auto_exposure(&mut input, width, height);
            self.timings.record("FilmingStage.auto_exposure", t0.elapsed().as_secs_f64());

            let t0 = Instant::now();
            let (cropped, cw, ch) = self.resize.crop_and_rescale(&input, width, height, 3);
            self.timings.record("ResizingService.crop_and_rescale", t0.elapsed().as_secs_f64());
            let px_size = self.resize.pixel_size_um();

            let t0 = Instant::now();
            let log_raw_film = self.filming.expose(&cropped, cw, ch, px_size, &mut self.lut_service);
            self.timings.record("FilmingStage.expose", t0.elapsed().as_secs_f64());
            if self.params.debug.debug_mode == DebugMode::Output && self.params.debug.output_film_log_raw {
                self.last_elapsed_time = Some(total_start.elapsed().as_secs_f64());
                return Ok(SimulationOutput { data: log_raw_film, width: cw, height: ch, channels: 3 });
            }

            let t0 = Instant::now();
            let cmy = self.filming.develop(&log_raw_film, cw, ch, px_size);
            self.timings.record("FilmingStage.develop", t0.elapsed().as_secs_f64());
            if self.params.debug.debug_mode == DebugMode::Output && self.params.debug.output_film_density_cmy {
                self.last_elapsed_time = Some(total_start.elapsed().as_secs_f64());
                return Ok(SimulationOutput { data: cmy, width: cw, height: ch, channels: 3 });
            }
            
            (cmy, cw, ch, px_size)
        };

        let final_rgb = if self.params.io.scan_film {
            let t0 = Instant::now();
            let rgb = self.scanning.scan(&cmy_film, w, h, &mut self.lut_service, &self.color_reference);
            self.timings.record("ScanningStage.scan", t0.elapsed().as_secs_f64());
            rgb
        } else {
            let bw_filming_corr = self.color_reference.black_white_filming_exposure_correction(&self.params.film);
            let midgray_density = self.filming.compute_midgray_spectral_density(bw_filming_corr);
            let midgray_density_comp = if self.params.camera.exposure_compensation_ev != 0.0 {
                let comp = 2.0_f64.powf(self.params.camera.exposure_compensation_ev);
                Some(self.filming.compute_midgray_spectral_density(bw_filming_corr * comp))
            } else {
                None
            };
            self.enlarger_service.set_density_spectral_midgray_pair(midgray_density, midgray_density_comp);

            let t0 = Instant::now();
            let (log_raw_print, log_raw_print_black, log_raw_print_white) = self.printing.expose(
                &cmy_film, w, h, pixel_size_um, &mut self.lut_service, &self.enlarger_service, &mut self.color_reference
            );
            self.timings.record("PrintingStage.expose", t0.elapsed().as_secs_f64());

            let t0 = Instant::now();
            let cmy_print = self.printing.develop(&log_raw_print);
            self.timings.record("PrintingStage.develop", t0.elapsed().as_secs_f64());
            
            self.color_reference.set_log_raw_print_references(log_raw_print_black, log_raw_print_white);
            let color_ref_clone = self.color_reference.clone();
            self.color_reference.update_negative_print_references(&self.params.print, &self.params.print_render, |cmy| self.scanning.cmy_to_log_xyz(cmy, &color_ref_clone));

            if self.params.debug.debug_mode == DebugMode::Output && self.params.debug.output_print_density_cmy {
                self.last_elapsed_time = Some(total_start.elapsed().as_secs_f64());
                return Ok(SimulationOutput { data: cmy_print, width: w, height: h, channels: 3 });
            }

            let t0 = Instant::now();
            let rgb = self.scanning.scan(&cmy_print, w, h, &mut self.lut_service, &self.color_reference);
            self.timings.record("ScanningStage.scan", t0.elapsed().as_secs_f64());
            rgb
        };

        self.last_elapsed_time = Some(total_start.elapsed().as_secs_f64());
        Ok(SimulationOutput { data: final_rgb, width: w, height: h, channels: 3 })
    }

    pub fn get_timings(&self) -> &Timings { &self.timings }
    pub fn get_total_elapsed_time(&self) -> Option<f64> { self.last_elapsed_time }
    pub fn format_timings(&self) -> String { self.timings.format(self.last_elapsed_time) }

    /// Generates a baked 3D LUT (Look-Up Table) of the current spectral color transformation.
    /// Returns a flat vector of f32 RGB values representing the 3D lattice.
    pub fn generate_lut(&self, lut_size: usize) -> Result<Vec<f32>, SimulationError> {
        let mut lut_params = self.params.clone();
        
        // 1. Disable all spatial effects (LUTs cannot encode spatial blur)
        lut_params.film_render.grain.active = false;
        lut_params.film_render.halation.active = false;
        lut_params.film_render.glare.active = false;
        lut_params.print_render.glare.active = false;
        lut_params.camera.diffusion_filter.active = false;
        lut_params.enlarger.diffusion_filter.active = false;
        lut_params.camera.lens_blur_um = 0.0;
        lut_params.enlarger.lens_blur = 0.0;
        lut_params.scanner.lens_blur = 0.0;
        
        // 2. Disable auto-exposure (scene brightness should be handled before the LUT)
        lut_params.camera.auto_exposure = false;
        
        // 3. Set LutGeneration mode to bypass cropping
        lut_params.debug.debug_mode = DebugMode::LutGeneration;
        
        let mut pipeline = SimulationPipeline::new(lut_params);
        let n_px = lut_size * lut_size * lut_size;
        let mut grid = vec![0.0f64; n_px * 3];
        
        // 4. Generate 3D RGB lattice (R fastest, then G, then B)
        let mut idx = 0;
        for b in 0..lut_size {
            for g in 0..lut_size {
                for r in 0..lut_size {
                    grid[idx] = r as f64 / (lut_size - 1) as f64;
                    grid[idx + 1] = g as f64 / (lut_size - 1) as f64;
                    grid[idx + 2] = b as f64 / (lut_size - 1) as f64;
                    idx += 3;
                }
            }
        }
        
        // 5. Process
        let out = pipeline.process(&grid, n_px, 1)?;
        
        // 6. Convert to f32
        let f32_out: Vec<f32> = out.data.into_iter().map(|v| (v as f32).clamp(0.0, 1.0)).collect();
        Ok(f32_out)
    }
}

#[derive(Debug, Clone)]
pub struct SimulationOutput {
    pub data: Vec<f64>,
    pub width: usize,
    pub height: usize,
    pub channels: usize,
}

#[derive(Debug, Clone)]
pub enum SimulationError {
    InvalidInput(String),
}

impl std::fmt::Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::InvalidInput(s) => write!(f, "invalid input: {s}") }
    }
}

impl std::error::Error for SimulationError {}
