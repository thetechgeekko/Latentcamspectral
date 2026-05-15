//! Serde-serialisable profile data structures.

use serde::{Deserialize, Serialize};

/// Top-level profile container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub info: ProfileInfo,
    pub data: ProfileData,
}

impl Profile {
    /// Returns true if this is a negative film/paper.
    pub fn is_negative(&self) -> bool { self.info.r#type == "negative" }
    /// Returns true if this is a positive (reversal) film.
    pub fn is_positive(&self) -> bool { self.info.r#type == "positive" }
    /// Returns true if the support is film.
    pub fn is_film(&self) -> bool { self.info.support == "film" }
    /// Returns true if the support is paper.
    pub fn is_paper(&self) -> bool { self.info.support == "paper" }
    /// Returns true if this is a filming-stage profile.
    pub fn is_filming(&self) -> bool { self.info.stage == "filming" }
    /// Returns true if this is a printing-stage profile.
    pub fn is_printing(&self) -> bool { self.info.stage == "printing" }
    /// Returns true if this is a still-photography film.
    pub fn is_still(&self) -> bool { self.info.r#use == "still" }
    /// Returns true if this is a cinema film.
    pub fn is_cine(&self) -> bool { self.info.r#use == "cine" }
    /// Returns true if colour (3-channel) rather than B&W.
    pub fn is_color(&self) -> bool { self.info.channel_model == "color" }
}

/// Metadata section of a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    pub stock: String,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub support: String,
    pub stage: String,
    #[serde(rename = "use")]
    pub r#use: String,
    pub antihalation: String,
    #[serde(default)]
    pub target_print: Option<String>,
    pub channel_model: String,
    pub densitometer: String,
    pub log_sensitivity_density_over_min: f64,
    pub reference_illuminant: String,
    pub viewing_illuminant: String,
    #[serde(default)]
    pub fitted_cmy_midscale_neutral_density: Option<Vec<f64>>,
    #[serde(default)]
    pub log_exposure_midscale_neutral: Option<f64>,
}

/// Numerical data section of a profile.
///
/// All spectral arrays are flattened row-major:
///   - `log_sensitivity`  : N_WL × 3   → len = N_WL * 3
///   - `channel_density`  : N_WL × 3   → len = N_WL * 3
///   - `density_curves`   : N_LE × 3   → len = N_LE * 3
///   - `density_curves_layers` : N_LE × 3 × 3 → len = N_LE * 9
///
/// `None` values in the JSON (NaN stand-ins) are replaced by `f64::NAN`
/// via the custom deserialiser below.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    /// Wavelength grid (nm), length = N_WL.
    pub wavelengths: Vec<f64>,
    /// Log spectral sensitivities, shape N_WL × 3 (flattened, row-major).
    #[serde(deserialize_with = "deser_nan_array")]
    pub log_sensitivity: Vec<f64>,
    /// Hanatos 2025 bandpass correction, shape N_WL × 3 (may be empty).
    #[serde(default, deserialize_with = "deser_nan_array_opt")]
    pub bandpass_hanatos2025: Vec<f64>,
    /// Spectral dye absorption per channel, shape N_WL × 3.
    #[serde(deserialize_with = "deser_nan_array")]
    pub channel_density: Vec<f64>,
    /// Film base + fog spectral density, length = N_WL.
    #[serde(deserialize_with = "deser_nan_vec")]
    pub base_density: Vec<f64>,
    /// Midscale neutral density spectrum, length = N_WL.
    #[serde(deserialize_with = "deser_nan_vec")]
    pub midscale_neutral_density: Vec<f64>,
    /// Log-exposure axis, length = N_LE.
    pub log_exposure: Vec<f64>,
    /// H&D density curves, shape N_LE × 3 (flattened).
    #[serde(deserialize_with = "deser_nan_array")]
    pub density_curves: Vec<f64>,
    /// Per-sublayer density curves, shape N_LE × 3 × 3 (flattened).
    #[serde(default, deserialize_with = "deser_nan_array_opt")]
    pub density_curves_layers: Vec<f64>,
}

impl ProfileData {
    /// Returns the number of wavelength samples.
    pub fn n_wl(&self) -> usize { self.wavelengths.len() }
    /// Returns the number of log-exposure samples.
    pub fn n_le(&self) -> usize { self.log_exposure.len() }

    /// Access `log_sensitivity[wl_idx, ch]`.
    pub fn log_sensitivity_at(&self, wl: usize, ch: usize) -> f64 {
        self.log_sensitivity[wl * 3 + ch]
    }
    /// Access `channel_density[wl_idx, ch]`.
    pub fn channel_density_at(&self, wl: usize, ch: usize) -> f64 {
        self.channel_density[wl * 3 + ch]
    }
    /// Access `density_curves[le_idx, ch]`.
    pub fn density_curve_at(&self, le: usize, ch: usize) -> f64 {
        self.density_curves[le * 3 + ch]
    }
    /// Access `density_curves_layers[le_idx, layer, ch]`.
    pub fn density_curve_layer_at(&self, le: usize, layer: usize, ch: usize) -> f64 {
        self.density_curves_layers[le * 9 + layer * 3 + ch]
    }
}

// ---------------------------------------------------------------------------
// Custom deserialisers: JSON `null` → f64::NAN
// ---------------------------------------------------------------------------

fn deser_nan_vec<'de, D>(d: D) -> Result<Vec<f64>, D::Error>
where D: serde::Deserializer<'de>
{
    let raw: Vec<Option<f64>> = Deserialize::deserialize(d)?;
    Ok(raw.into_iter().map(|v| v.unwrap_or(f64::NAN)).collect())
}

fn deser_nan_array<'de, D>(d: D) -> Result<Vec<f64>, D::Error>
where D: serde::Deserializer<'de>
{
    // Accepts arbitrarily nested arrays of numbers/nulls, because profile
    // payloads contain 1-D, 2-D and 3-D numeric tensors.
    use serde::de::{SeqAccess, Visitor};
    use std::fmt;

    fn flatten_value(value: serde_json::Value, out: &mut Vec<f64>) {
        match value {
            serde_json::Value::Array(items) => {
                for item in items {
                    flatten_value(item, out);
                }
            }
            serde_json::Value::Number(n) => out.push(n.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::Null => out.push(f64::NAN),
            _ => out.push(f64::NAN),
        }
    }

    struct NanArrayVisitor;

    impl<'de> Visitor<'de> for NanArrayVisitor {
        type Value = Vec<f64>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a nested array of numbers or nulls")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(elem) = seq.next_element::<serde_json::Value>()? {
                flatten_value(elem, &mut out);
            }
            Ok(out)
        }
    }

    d.deserialize_seq(NanArrayVisitor)
}

fn deser_nan_array_opt<'de, D>(d: D) -> Result<Vec<f64>, D::Error>
where D: serde::Deserializer<'de>
{
    deser_nan_array(d)
}
