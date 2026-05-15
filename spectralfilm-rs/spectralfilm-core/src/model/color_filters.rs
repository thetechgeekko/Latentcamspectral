//! Dichroic CMY enlarger filters, generic optical filters, and bandpass filters.
//!
//! Mirrors `spektrafilm/model/color_filters.py` and the CSV loading helpers in
//! `spektrafilm/utils/io.py`.  The Python implementation loads package resources
//! at runtime; this Rust port embeds the copied CSV assets with `include_str!` so
//! Android builds do not depend on filesystem access to resolve filter data.

use crate::config::{N_WL, WL_START, WL_STEP};

// ---------------------------------------------------------------------------
// Embedded filter CSV assets
// ---------------------------------------------------------------------------

const THORLABS_C_CSV: &str = include_str!("../../assets/data/filters/dichroics/thorlabs/filter_c.csv");
const THORLABS_M_CSV: &str = include_str!("../../assets/data/filters/dichroics/thorlabs/filter_m.csv");
const THORLABS_Y_CSV: &str = include_str!("../../assets/data/filters/dichroics/thorlabs/filter_y.csv");

const EDMUND_OPTICS_C_CSV: &str = include_str!("../../assets/data/filters/dichroics/edmund_optics/filter_c.csv");
const EDMUND_OPTICS_M_CSV: &str = include_str!("../../assets/data/filters/dichroics/edmund_optics/filter_m.csv");
const EDMUND_OPTICS_Y_CSV: &str = include_str!("../../assets/data/filters/dichroics/edmund_optics/filter_y.csv");

const DURST_DIGITAL_LIGHT_C_CSV: &str = include_str!("../../assets/data/filters/dichroics/durst_digital_light/filter_c.csv");
const DURST_DIGITAL_LIGHT_M_CSV: &str = include_str!("../../assets/data/filters/dichroics/durst_digital_light/filter_m.csv");
const DURST_DIGITAL_LIGHT_Y_CSV: &str = include_str!("../../assets/data/filters/dichroics/durst_digital_light/filter_y.csv");

const SCHOTT_KG1_CSV: &str = include_str!("../../assets/data/filters/heat_absorbing/schott/KG1.csv");
const SCHOTT_KG3_CSV: &str = include_str!("../../assets/data/filters/heat_absorbing/schott/KG3.csv");
const SCHOTT_KG5_CSV: &str = include_str!("../../assets/data/filters/heat_absorbing/schott/KG5.csv");

const CANON_24_F28_IS_CSV: &str = include_str!("../../assets/data/filters/lens_transmission/canon/canon_24_f28_is.csv");

// ---------------------------------------------------------------------------
// Bandpass (UV/IR cut) filter
// ---------------------------------------------------------------------------

/// Compute a bandpass transmittance array (length N_WL).
///
/// `filter_uv` = (amplitude, center_nm, width_nm) — blocks UV below center.
/// `filter_ir` = (amplitude, center_nm, width_nm) — blocks IR above center.
pub fn compute_band_pass_filter(
    filter_uv: (f64, f64, f64),
    filter_ir: (f64, f64, f64),
) -> [f64; N_WL] {
    let (amp_uv, wl_uv, w_uv) = filter_uv;
    let (amp_ir, wl_ir, w_ir) = filter_ir;
    let amp_uv = amp_uv.clamp(0.0, 1.0);
    let amp_ir = amp_ir.clamp(0.0, 1.0);

    let mut out = [1.0f64; N_WL];
    for i in 0..N_WL {
        let wl = WL_START + i as f64 * WL_STEP;
        let uv = 1.0 - amp_uv + amp_uv * sigmoid_erf(wl, wl_uv, w_uv);
        let ir = 1.0 - amp_ir + amp_ir * sigmoid_erf(wl, wl_ir, -w_ir);
        out[i] = uv * ir;
    }
    out
}

fn sigmoid_erf(x: f64, center: f64, width: f64) -> f64 {
    let denom = if width.abs() < 1e-10 {
        if width.is_sign_negative() { -1e-10 } else { 1e-10 }
    } else {
        width
    };
    let z = (x - center) / denom;
    erf(z) * 0.5 + 0.5
}

/// Rational approximation of erf (Abramowitz & Stegun 7.1.26).
#[inline]
pub fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = t * (0.254829592
        + t * (-0.284496736
        + t * (1.421413741
        + t * (-1.453152027
        + t * 1.061405429))));
    sign * (1.0 - poly * (-x * x).exp())
}

// ---------------------------------------------------------------------------
// Dichroic CMY filters
// ---------------------------------------------------------------------------

/// Pre-computed custom dichroic CMY filter transmittances at the standard 5 nm
/// grid. Each row is [C-filter, M-filter, Y-filter] transmittance at that
/// wavelength. Based on the Python `custom_dichroic_filters` erf sigmoid model.
pub fn custom_dichroic_filters() -> [[f64; 3]; N_WL] {
    create_combined_dichroic_filter([516.0, 500.0, 610.0, 607.0], [12.0, 8.0, 8.0, 8.0])
}

/// Rust equivalent of Python `create_combined_dichroic_filter` on the standard
/// wavelength grid. `edges` and `transitions` use the Python order:
/// [Y edge, M low edge, M high edge, C edge].
pub fn create_combined_dichroic_filter(
    edges: [f64; 4],
    transitions: [f64; 4],
) -> [[f64; 3]; N_WL] {
    let mut filters = [[0.0f64; 3]; N_WL];
    for i in 0..N_WL {
        let wl = WL_START + i as f64 * WL_STEP;
        filters[i][2] = erf((wl - edges[0]) / transitions[0]) * 0.5 + 0.5;
        filters[i][1] = if wl <= 550.0 {
            -erf((wl - edges[1]) / transitions[1]) * 0.5 + 0.5
        } else {
            erf((wl - edges[2]) / transitions[2]) * 0.5 + 0.5
        };
        filters[i][0] = -erf((wl - edges[3]) / transitions[3]) * 0.5 + 0.5;
    }
    filters
}

/// Return dichroic CMY filters for a supported brand.
///
/// Supported brands mirror Python: `custom`, `thorlabs`, `edmund_optics`, and
/// `durst_digital_light`. Unknown brands log a warning and fall back to custom.
///
/// Remaining limitation: Python uses SciPy's Akima1DInterpolator. This Rust port
/// intentionally uses monotonic linear interpolation over the embedded CSV data
/// to keep the core dependency-free and Android-friendly. The sampled 5 nm grid
/// is dense enough for the bundled filter curves, but it is not bit-identical to
/// SciPy Akima around sparse or very sharp measured points.
pub fn dichroic_filters_for_brand(brand: &str) -> [[f64; 3]; N_WL] {
    match brand {
        "custom" => custom_dichroic_filters(),
        "thorlabs" => load_dichroic_filters_from_csv(THORLABS_C_CSV, THORLABS_M_CSV, THORLABS_Y_CSV),
        "edmund_optics" => load_dichroic_filters_from_csv(EDMUND_OPTICS_C_CSV, EDMUND_OPTICS_M_CSV, EDMUND_OPTICS_Y_CSV),
        "durst_digital_light" => load_dichroic_filters_from_csv(
            DURST_DIGITAL_LIGHT_C_CSV,
            DURST_DIGITAL_LIGHT_M_CSV,
            DURST_DIGITAL_LIGHT_Y_CSV,
        ),
        _ => {
            log::warn!("Unknown dichroic filter brand '{}', falling back to custom", brand);
            custom_dichroic_filters()
        }
    }
}

/// Default Python-compatible dichroic set (`brand='thorlabs'`).
pub fn dichroic_filters() -> [[f64; 3]; N_WL] {
    thorlabs_dichroic_filters()
}

pub fn thorlabs_dichroic_filters() -> [[f64; 3]; N_WL] {
    dichroic_filters_for_brand("thorlabs")
}

pub fn edmund_optics_dichroic_filters() -> [[f64; 3]; N_WL] {
    dichroic_filters_for_brand("edmund_optics")
}

pub fn durst_digital_light_dichroic_filters() -> [[f64; 3]; N_WL] {
    dichroic_filters_for_brand("durst_digital_light")
}

/// Backward-compatible misspelling matching the Python module variable name.
pub fn durst_digital_light_dicrhoic_filters() -> [[f64; 3]; N_WL] {
    durst_digital_light_dichroic_filters()
}

fn load_dichroic_filters_from_csv(c_csv: &str, m_csv: &str, y_csv: &str) -> [[f64; 3]; N_WL] {
    let c = interpolate_csv_to_grid(c_csv, 100.0);
    let m = interpolate_csv_to_grid(m_csv, 100.0);
    let y = interpolate_csv_to_grid(y_csv, 100.0);
    let mut filters = [[0.0f64; 3]; N_WL];
    for i in 0..N_WL {
        filters[i] = [c[i], m[i], y[i]];
    }
    filters
}

/// Apply raw dichroic transmittance controls to a light source spectrum.
///
/// `filter_transmittance_values` are in CMY order and follow the Python formula:
/// `dimmed = 1 - (1 - filter) * (1 - value)`.
pub fn apply_dichroic_filters(
    light: &[f64; N_WL],
    filters: &[[f64; 3]; N_WL],
    filter_transmittance_values: &[f64; 3],
) -> [f64; N_WL] {
    let mut out = [0.0f64; N_WL];
    for i in 0..N_WL {
        let dim_c = 1.0 - (1.0 - filters[i][0]) * (1.0 - filter_transmittance_values[0]);
        let dim_m = 1.0 - (1.0 - filters[i][1]) * (1.0 - filter_transmittance_values[1]);
        let dim_y = 1.0 - (1.0 - filters[i][2]) * (1.0 - filter_transmittance_values[2]);
        out[i] = light[i] * dim_c * dim_m * dim_y;
    }
    out
}

/// Apply dichroic CMY enlarger filters in Kodak CC units with explicit filter curves.
pub fn apply_dichroic_filters_cc(
    light: &[f64; N_WL],
    filters: &[[f64; 3]; N_WL],
    filter_cc: &[f64; 3],
) -> [f64; N_WL] {
    let t = cc_to_transmittance(filter_cc);
    apply_dichroic_filters(light, filters, &t)
}

/// Apply dichroic CMY enlarger filters to a light source spectrum using the
/// historical Rust default (`custom`) to preserve existing output behavior.
///
/// `filter_cc` is in CMY Kodak CC units (100 units = 1.0 density = 90% reduction).
pub fn color_enlarger(light: &[f64; N_WL], filter_cc: &[f64; 3]) -> [f64; N_WL] {
    let dichroics = custom_dichroic_filters();
    apply_dichroic_filters_cc(light, &dichroics, filter_cc)
}

/// Brand-selectable variant of `color_enlarger` for parity with Python's
/// `DichroicFilters(brand=...)` support.
pub fn color_enlarger_with_brand(
    light: &[f64; N_WL],
    filter_cc: &[f64; 3],
    brand: &str,
) -> [f64; N_WL] {
    let dichroics = dichroic_filters_for_brand(brand);
    apply_dichroic_filters_cc(light, &dichroics, filter_cc)
}

fn cc_to_transmittance(filter_cc: &[f64; 3]) -> [f64; 3] {
    [
        10.0_f64.powf(-filter_cc[0] / 100.0),
        10.0_f64.powf(-filter_cc[1] / 100.0),
        10.0_f64.powf(-filter_cc[2] / 100.0),
    ]
}

// ---------------------------------------------------------------------------
// Generic heat-absorbing and lens-transmission filters
// ---------------------------------------------------------------------------

/// Return a generic filter transmittance curve by `(name, brand, filter_type)`.
///
/// Supported CSV-backed combinations currently embedded:
/// - heat_absorbing / schott / KG1, KG3, KG5
/// - lens_transmission / canon / canon_24_f28_is
///
/// `data_in_percentage` mirrors Python's `percent_transmittance` argument. For
/// the bundled Canon lens data this should be `true`; Schott KG data already uses
/// fractional transmittance.
pub fn generic_filter_transmittance(
    name: &str,
    brand: &str,
    filter_type: &str,
    data_in_percentage: bool,
) -> [f64; N_WL] {
    let scale = if data_in_percentage { 100.0 } else { 1.0 };
    match (filter_type, brand, name) {
        ("heat_absorbing", "schott", "KG1") => interpolate_csv_to_grid(SCHOTT_KG1_CSV, scale),
        ("heat_absorbing", "schott", "KG3") => interpolate_csv_to_grid(SCHOTT_KG3_CSV, scale),
        ("heat_absorbing", "schott", "KG5") => interpolate_csv_to_grid(SCHOTT_KG5_CSV, scale),
        ("lens_transmission", "canon", "canon_24_f28_is") => interpolate_csv_to_grid(CANON_24_F28_IS_CSV, scale),
        _ => {
            log::warn!(
                "Unknown filter '{}/{}/{}', falling back to unity transmittance",
                filter_type,
                brand,
                name,
            );
            [1.0f64; N_WL]
        }
    }
}

pub fn schott_kg1_heat_filter() -> [f64; N_WL] {
    generic_filter_transmittance("KG1", "schott", "heat_absorbing", false)
}

pub fn schott_kg3_heat_filter() -> [f64; N_WL] {
    generic_filter_transmittance("KG3", "schott", "heat_absorbing", false)
}

pub fn schott_kg5_heat_filter() -> [f64; N_WL] {
    generic_filter_transmittance("KG5", "schott", "heat_absorbing", false)
}

/// Backward/semantic alias used by illuminant code and older callers.
pub fn schott_kg3_transmittance() -> [f64; N_WL] {
    schott_kg3_heat_filter()
}

/// Python-compatible `generic_lens_transmission` using Canon 24mm f/2.8 IS data.
pub fn generic_lens_transmission() -> [f64; N_WL] {
    generic_filter_transmittance("canon_24_f28_is", "canon", "lens_transmission", true)
}

pub fn lens_transmittance(name: &str, brand: &str, data_in_percentage: bool) -> [f64; N_WL] {
    generic_filter_transmittance(name, brand, "lens_transmission", data_in_percentage)
}

/// Apply a generic filter using Python's `GenericFilter.apply` formula:
/// `dimmed_filter = 1 - (1 - transmittance) * value`.
pub fn apply_generic_filter(
    light: &[f64; N_WL],
    transmittance: &[f64; N_WL],
    value: f64,
) -> [f64; N_WL] {
    let mut out = [0.0f64; N_WL];
    for i in 0..N_WL {
        let dimmed = 1.0 - (1.0 - transmittance[i]) * value;
        out[i] = light[i] * dimmed;
    }
    out
}

// ---------------------------------------------------------------------------
// Embedded CSV interpolation helpers
// ---------------------------------------------------------------------------

fn interpolate_csv_to_grid(csv: &str, scale: f64) -> [f64; N_WL] {
    let points = parse_csv_points_unique(csv, scale);
    interpolate_points_to_grid(&points)
}

fn parse_csv_points_unique(csv: &str, scale: f64) -> Vec<(f64, f64)> {
    let mut points: Vec<(f64, f64)> = Vec::new();
    for line in csv.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split(',');
        let Some(wl_s) = parts.next() else { continue };
        let Some(v_s) = parts.next() else { continue };
        let Ok(wl) = wl_s.trim().parse::<f64>() else { continue };
        let Ok(value) = v_s.trim().parse::<f64>() else { continue };
        if points.iter().any(|(existing_wl, _)| (*existing_wl - wl).abs() < 1e-9) {
            // Match Python's `np.unique(..., return_index=True)` behaviour: keep
            // the first sample when digitised CSV assets contain duplicate wavelengths.
            continue;
        }
        points.push((wl, value / scale));
    }
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    points
}

fn interpolate_points_to_grid(points: &[(f64, f64)]) -> [f64; N_WL] {
    let mut out = [1.0f64; N_WL];
    if points.is_empty() {
        return out;
    }
    if points.len() == 1 {
        out.fill(points[0].1.clamp(0.0, 1.0));
        return out;
    }

    let mut segment = 0usize;
    for i in 0..N_WL {
        let wl = WL_START + i as f64 * WL_STEP;
        while segment + 1 < points.len() && points[segment + 1].0 < wl {
            segment += 1;
        }

        let value = if wl <= points[0].0 {
            points[0].1
        } else if wl >= points[points.len() - 1].0 {
            points[points.len() - 1].1
        } else {
            let next = (segment + 1).min(points.len() - 1);
            let (x0, y0) = points[segment];
            let (x1, y1) = points[next];
            if (x1 - x0).abs() < 1e-12 {
                y0
            } else {
                let t = (wl - x0) / (x1 - x0);
                y0 + (y1 - y0) * t
            }
        };
        out[i] = value.clamp(0.0, 1.0);
    }
    out
}
