//! Profile data structures and JSON I/O.
//!
//! Mirrors `spektrafilm/profiles/io.py`.

mod schema;
pub use schema::{Profile, ProfileData, ProfileInfo};

use serde_json;
use std::io;

/// Built-in profile stock identifiers embedded in this crate.
pub const BUILT_IN_PROFILES: &[&str] = &[
    "fujifilm_c200",
    "fujifilm_crystal_archive_typeii",
    "fujifilm_pro_400h",
    "fujifilm_provia_100f",
    "fujifilm_velvia_100",
    "fujifilm_xtra_400",
    "kodak_2383",
    "kodak_2393",
    "kodak_ektachrome_100",
    "kodak_ektacolor_edge",
    "kodak_ektar_100",
    "kodak_endura_premier",
    "kodak_gold_200",
    "kodak_kodachrome_64",
    "kodak_portra_160",
    "kodak_portra_400",
    "kodak_portra_800",
    "kodak_portra_800_push1",
    "kodak_portra_800_push2",
    "kodak_portra_endura",
    "kodak_supra_endura",
    "kodak_ultra_endura",
    "kodak_ultramax_400",
    "kodak_verita_200d",
    "kodak_vision3_200t",
    "kodak_vision3_250d",
    "kodak_vision3_500t",
    "kodak_vision3_50d",
];

/// Return the built-in profile stock identifiers available to `load_profile`.
pub fn available_profiles() -> &'static [&'static str] {
    BUILT_IN_PROFILES
}

/// Load one of the built-in profiles by stock identifier.
///
/// The `stock` argument is the profile's `info.stock` value without the `.json`
/// suffix, matching the Python `load_profile(stock)` helper.
pub fn load_profile(stock: &str) -> Result<Profile, ProfileError> {
    let json = built_in_profile_json(stock).ok_or_else(|| ProfileError::NotFound(stock.to_string()))?;
    profile_from_json(json)
}

/// Split-architecture alias matching the Python API.
pub fn load_processed_profile(stock: &str) -> Result<Profile, ProfileError> {
    load_profile(stock)
}

fn built_in_profile_json(stock: &str) -> Option<&'static str> {
    match stock {
        "fujifilm_c200" => Some(include_str!("../../assets/data/profiles/fujifilm_c200.json")),
        "fujifilm_crystal_archive_typeii" => Some(include_str!("../../assets/data/profiles/fujifilm_crystal_archive_typeii.json")),
        "fujifilm_pro_400h" => Some(include_str!("../../assets/data/profiles/fujifilm_pro_400h.json")),
        "fujifilm_provia_100f" => Some(include_str!("../../assets/data/profiles/fujifilm_provia_100f.json")),
        "fujifilm_velvia_100" => Some(include_str!("../../assets/data/profiles/fujifilm_velvia_100.json")),
        "fujifilm_xtra_400" => Some(include_str!("../../assets/data/profiles/fujifilm_xtra_400.json")),
        "kodak_2383" => Some(include_str!("../../assets/data/profiles/kodak_2383.json")),
        "kodak_2393" => Some(include_str!("../../assets/data/profiles/kodak_2393.json")),
        "kodak_ektachrome_100" => Some(include_str!("../../assets/data/profiles/kodak_ektachrome_100.json")),
        "kodak_ektacolor_edge" => Some(include_str!("../../assets/data/profiles/kodak_ektacolor_edge.json")),
        "kodak_ektar_100" => Some(include_str!("../../assets/data/profiles/kodak_ektar_100.json")),
        "kodak_endura_premier" => Some(include_str!("../../assets/data/profiles/kodak_endura_premier.json")),
        "kodak_gold_200" => Some(include_str!("../../assets/data/profiles/kodak_gold_200.json")),
        "kodak_kodachrome_64" => Some(include_str!("../../assets/data/profiles/kodak_kodachrome_64.json")),
        "kodak_portra_160" => Some(include_str!("../../assets/data/profiles/kodak_portra_160.json")),
        "kodak_portra_400" => Some(include_str!("../../assets/data/profiles/kodak_portra_400.json")),
        "kodak_portra_800" => Some(include_str!("../../assets/data/profiles/kodak_portra_800.json")),
        "kodak_portra_800_push1" => Some(include_str!("../../assets/data/profiles/kodak_portra_800_push1.json")),
        "kodak_portra_800_push2" => Some(include_str!("../../assets/data/profiles/kodak_portra_800_push2.json")),
        "kodak_portra_endura" => Some(include_str!("../../assets/data/profiles/kodak_portra_endura.json")),
        "kodak_supra_endura" => Some(include_str!("../../assets/data/profiles/kodak_supra_endura.json")),
        "kodak_ultra_endura" => Some(include_str!("../../assets/data/profiles/kodak_ultra_endura.json")),
        "kodak_ultramax_400" => Some(include_str!("../../assets/data/profiles/kodak_ultramax_400.json")),
        "kodak_verita_200d" => Some(include_str!("../../assets/data/profiles/kodak_verita_200d.json")),
        "kodak_vision3_200t" => Some(include_str!("../../assets/data/profiles/kodak_vision3_200t.json")),
        "kodak_vision3_250d" => Some(include_str!("../../assets/data/profiles/kodak_vision3_250d.json")),
        "kodak_vision3_500t" => Some(include_str!("../../assets/data/profiles/kodak_vision3_500t.json")),
        "kodak_vision3_50d" => Some(include_str!("../../assets/data/profiles/kodak_vision3_50d.json")),
        _ => None,
    }
}

/// Load a profile from a JSON string (e.g. embedded via `include_str!`).
pub fn profile_from_json(json: &str) -> Result<Profile, ProfileError> {
    let p: Profile = serde_json::from_str(json).map_err(ProfileError::Json)?;
    validate_profile(&p)?;
    Ok(p)
}

/// Load a profile from raw JSON bytes.
pub fn profile_from_bytes(bytes: &[u8]) -> Result<Profile, ProfileError> {
    let p: Profile = serde_json::from_slice(bytes).map_err(ProfileError::Json)?;
    validate_profile(&p)?;
    Ok(p)
}

/// Serialise a profile back to a pretty-printed JSON string.
pub fn profile_to_json(p: &Profile) -> Result<String, ProfileError> {
    serde_json::to_string_pretty(p).map_err(ProfileError::Json)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate shape consistency of a loaded profile.
fn validate_profile(p: &Profile) -> Result<(), ProfileError> {
    let d = &p.data;
    let nwl = d.wavelengths.len();
    let nle = d.log_exposure.len();

    let ok = !d.wavelengths.is_empty()
        && d.log_sensitivity.len() == nwl * 3
        && d.channel_density.len() == nwl * 3
        && d.base_density.len() == nwl
        && d.midscale_neutral_density.len() == nwl
        && d.density_curves.len() == nle * 3
        && !d.log_exposure.is_empty();

    if !ok {
        return Err(ProfileError::InvalidShape(p.info.stock.clone()));
    }

    // Validate enum fields
    let valid_types = ["negative", "positive"];
    let valid_supports = ["film", "paper"];
    let valid_stages = ["filming", "printing"];
    let valid_uses = ["still", "cine"];
    let valid_ah = ["strong", "weak", "no"];
    let valid_cm = ["color", "bw"];

    macro_rules! check {
        ($val:expr, $set:expr, $field:literal) => {
            if !$set.contains(&$val.as_str()) {
                return Err(ProfileError::InvalidField {
                    stock: p.info.stock.clone(),
                    field: $field,
                    value: $val.clone(),
                });
            }
        };
    }
    check!(p.info.r#type, valid_types, "type");
    check!(p.info.support, valid_supports, "support");
    check!(p.info.stage, valid_stages, "stage");
    check!(p.info.r#use, valid_uses, "use");
    check!(p.info.antihalation, valid_ah, "antihalation");
    check!(p.info.channel_model, valid_cm, "channel_model");

    Ok(())
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ProfileError {
    Json(serde_json::Error),
    Io(io::Error),
    InvalidShape(String),
    InvalidField { stock: String, field: &'static str, value: String },
    NotFound(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(e) => write!(f, "JSON parse error: {e}"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::InvalidShape(s) => write!(f, "Profile '{s}': array shape mismatch"),
            Self::InvalidField { stock, field, value } => {
                write!(f, "Profile '{stock}': invalid {field}='{value}'")
            }
            Self::NotFound(s) => write!(f, "Profile '{s}' not found"),
        }
    }
}

impl std::error::Error for ProfileError {}
