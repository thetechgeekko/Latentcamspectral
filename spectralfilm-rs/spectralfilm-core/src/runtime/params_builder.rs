//! Runtime parameter digest helpers.

use crate::profiles::{load_profile, ProfileError};
use crate::runtime::params::RuntimePhotoParams;
use std::collections::HashMap;

const NEUTRAL_PRINT_FILTERS_JSON: &str = include_str!("../../assets/data/filters/neutral_print_filters.json");

/// Neutral print filter database keyed by print stock, enlarger illuminant and film stock.
pub type NeutralPrintFilterDatabase = HashMap<String, HashMap<String, HashMap<String, [f64; 3]>>>;

/// Parse a neutral print filter database JSON payload.
pub fn neutral_print_filters_from_json(json: &str) -> Result<NeutralPrintFilterDatabase, serde_json::Error> {
    serde_json::from_str(json)
}

/// Load the built-in neutral print filter database embedded in this crate.
pub fn neutral_print_filters_database() -> Result<NeutralPrintFilterDatabase, serde_json::Error> {
    neutral_print_filters_from_json(NEUTRAL_PRINT_FILTERS_JSON)
}

/// Apply neutral enlarger CMY filter values from a filter database when enabled.
pub fn apply_database_neutral_print_filters_with_database(
    mut params: RuntimePhotoParams,
    database: &NeutralPrintFilterDatabase,
    warn_missing: bool,
) -> RuntimePhotoParams {
    if !params.settings.neutral_print_filters_from_database {
        return params;
    }

    let stock_filters = database
        .get(params.print.info.stock.as_str())
        .and_then(|by_illuminant| by_illuminant.get(params.enlarger.illuminant.as_str()))
        .and_then(|by_film| by_film.get(params.film.info.stock.as_str()));

    if let Some([c_filter, m_filter, y_filter]) = stock_filters {
        params.enlarger.c_filter_neutral = *c_filter;
        params.enlarger.m_filter_neutral = *m_filter;
        params.enlarger.y_filter_neutral = *y_filter;
    } else if warn_missing {
        log::warn!(
            "No neutral print filters found in database for print stock {} with illuminant {} and film stock {}. Using defaults.",
            params.print.info.stock,
            params.enlarger.illuminant,
            params.film.info.stock
        );
    }

    params
}

/// Apply neutral enlarger CMY filter values from the built-in database when enabled.
pub fn apply_database_neutral_print_filters(params: RuntimePhotoParams) -> RuntimePhotoParams {
    match neutral_print_filters_database() {
        Ok(database) => apply_database_neutral_print_filters_with_database(params, &database, true),
        Err(err) => {
            log::warn!("Could not parse built-in neutral print filter database: {err}");
            params
        }
    }
}

/// Build runtime parameters from built-in film and print profile stock identifiers.
pub fn init_params(film_profile: &str, print_profile: &str) -> Result<RuntimePhotoParams, ProfileError> {
    Ok(RuntimePhotoParams::new(load_profile(film_profile)?, load_profile(print_profile)?))
}

/// Build runtime parameters using the Python defaults.
pub fn init_default_params() -> Result<RuntimePhotoParams, ProfileError> {
    init_params("kodak_portra_400", "kodak_portra_endura")
}

/// Apply stock-specific and debug/preview simplifications.
pub fn digest_params(mut params: RuntimePhotoParams) -> RuntimePhotoParams {
    params = apply_database_neutral_print_filters(params);

    if params.settings.preview_mode {
        params.enlarger.lens_blur = 0.0;
        params.film_render.dir_couplers.diffusion_size_um = 0.0;
        params.film_render.grain.active = false;
        params.film_render.grain.agx_particle_area_um2 = 0.0;
        params.film_render.grain.blur = 0.0;
        params.print_render.glare.blur = 0.0;
        params.camera.lens_blur_um = 0.0;
        params.scanner.lens_blur = 0.0;
        params.scanner.unsharp_mask = (0.0, 0.0);
    }

    apply_film_specifics(&mut params);

    if params.debug.deactivate_spatial_effects {
        params.film_render.halation.scatter_core_um = [0.0, 0.0, 0.0];
        params.film_render.halation.scatter_tail_um = [0.0, 0.0, 0.0];
        params.film_render.halation.halation_first_sigma_um = [0.0, 0.0, 0.0];
        params.film_render.dir_couplers.diffusion_size_um = 0.0;
        params.film_render.grain.blur = 0.0;
        params.film_render.grain.blur_dye_clouds_um = 0.0;
        params.print_render.glare.blur = 0.0;
        params.camera.lens_blur_um = 0.0;
        params.enlarger.lens_blur = 0.0;
        params.enlarger.diffusion_filter.active = false;
        params.camera.diffusion_filter.active = false;
        params.scanner.lens_blur = 0.0;
        params.scanner.unsharp_mask = (0.0, 0.0);
    }

    if params.debug.deactivate_stochastic_effects {
        params.film_render.grain.active = false;
        params.print_render.glare.active = false;
    }

    params
}

fn apply_film_specifics(params: &mut RuntimePhotoParams) {
    if params.film.is_positive() {
        params.film_render.dir_couplers.gamma_samelayer_rgb = [0.12, 0.08, 0.06];
        params.film_render.dir_couplers.gamma_interlayer_r_to_gb = [0.12, 0.06];
        params.film_render.dir_couplers.gamma_interlayer_g_to_rb = [0.08, 0.06];
        params.film_render.dir_couplers.gamma_interlayer_b_to_rg = [0.06, 0.06];
    }
    if params.film.is_negative() {
        params.film_render.dir_couplers.gamma_samelayer_rgb = [0.336, 0.319, 0.273];
        params.film_render.dir_couplers.gamma_interlayer_r_to_gb = [0.353, 0.302];
        params.film_render.dir_couplers.gamma_interlayer_g_to_rb = [0.154, 0.353];
        params.film_render.dir_couplers.gamma_interlayer_b_to_rg = [0.168, 0.226];
    }
    apply_halation_preset(params);

    match params.film.info.stock.as_str() {
        "fujifilm_velvia_100" => {
            params.film_render.dir_couplers.gamma_samelayer_rgb = [0.108, 0.072, 0.054];
            params.film_render.dir_couplers.gamma_interlayer_r_to_gb = [0.108, 0.054];
            params.film_render.dir_couplers.gamma_interlayer_g_to_rb = [0.072, 0.054];
            params.film_render.dir_couplers.gamma_interlayer_b_to_rg = [0.054, 0.054];
        }
        "fujifilm_provia_100f" => {
            params.film_render.dir_couplers.gamma_samelayer_rgb = [0.156, 0.104, 0.078];
            params.film_render.dir_couplers.gamma_interlayer_r_to_gb = [0.156, 0.078];
            params.film_render.dir_couplers.gamma_interlayer_g_to_rb = [0.104, 0.078];
            params.film_render.dir_couplers.gamma_interlayer_b_to_rg = [0.078, 0.078];
        }
        _ => {}
    }
}

fn apply_halation_preset(params: &mut RuntimePhotoParams) {
    if !params.film.is_film() { return; }
    let use_tag = params.film.info.r#use.as_str();
    let ah = params.film.info.antihalation.as_str();
    let sigma = if use_tag == "cine" { [50.0, 50.0, 50.0] } else { [65.0, 65.0, 65.0] };
    let strength = match ah {
        "strong" => [0.015, 0.005, 0.0],
        "weak" => [0.08, 0.02, 0.0],
        "no" => [0.30, 0.10, 0.015],
        _ => return,
    };
    params.film_render.halation.halation_first_sigma_um = sigma;
    params.film_render.halation.halation_strength = strength;
}
