//! Runtime parameter schema matching the Python `RuntimePhotoParams` model.

use serde::de::DeserializeOwned;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::model::couplers::DirCouplersParams;
use crate::model::diffusion::{DiffusionFilterParams, HalationParams};
use crate::model::glare::GlareParams;
use crate::model::grain::GrainParams;
use crate::profiles::Profile;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraParams {
    pub exposure_compensation_ev: f64,
    pub auto_exposure: bool,
    pub auto_exposure_method: String,
    pub lens_blur_um: f64,
    pub film_format_mm: f64,
    pub filter_uv: (f64, f64, f64),
    pub filter_ir: (f64, f64, f64),
    pub diffusion_filter: DiffusionFilterParams,
}

impl Default for CameraParams {
    fn default() -> Self {
        Self {
            exposure_compensation_ev: 0.0,
            auto_exposure: true,
            auto_exposure_method: "center_weighted".into(),
            lens_blur_um: 0.0,
            film_format_mm: 35.0,
            filter_uv: (0.0, 410.0, 8.0),
            filter_ir: (0.0, 675.0, 15.0),
            diffusion_filter: DiffusionFilterParams::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EnlargerParams {
    pub illuminant: String,
    pub print_exposure: f64,
    pub print_exposure_compensation: bool,
    pub normalize_print_exposure: bool,
    pub y_filter_shift: f64,
    pub m_filter_shift: f64,
    pub y_filter_neutral: f64,
    pub m_filter_neutral: f64,
    pub c_filter_neutral: f64,
    pub lens_blur: f64,
    pub diffusion_filter: DiffusionFilterParams,
    pub preflash_exposure: f64,
    pub preflash_y_filter_shift: f64,
    pub preflash_m_filter_shift: f64,
}

impl Default for EnlargerParams {
    fn default() -> Self {
        Self {
            illuminant: "TH-KG3".into(),
            print_exposure: 1.0,
            print_exposure_compensation: true,
            normalize_print_exposure: true,
            y_filter_shift: 0.0,
            m_filter_shift: 0.0,
            y_filter_neutral: 55.0,
            m_filter_neutral: 65.0,
            c_filter_neutral: 0.0,
            lens_blur: 0.0,
            diffusion_filter: DiffusionFilterParams::default(),
            preflash_exposure: 0.0,
            preflash_y_filter_shift: 0.0,
            preflash_m_filter_shift: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScannerParams {
    pub lens_blur: f64,
    pub white_correction: bool,
    pub black_correction: bool,
    pub white_level: f64,
    pub black_level: f64,
    pub unsharp_mask: (f64, f64),
}

impl Default for ScannerParams {
    fn default() -> Self {
        Self {
            lens_blur: 0.0,
            white_correction: false,
            black_correction: false,
            white_level: 0.98,
            black_level: 0.01,
            unsharp_mask: (0.7, 0.7),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FilmRenderingParams {
    pub density_curve_gamma: f64,
    pub grain: GrainParams,
    pub halation: HalationParams,
    pub dir_couplers: DirCouplersParams,
    pub glare: GlareParams,
}

impl Default for FilmRenderingParams {
    fn default() -> Self {
        Self {
            density_curve_gamma: 1.0,
            grain: GrainParams::default(),
            halation: HalationParams::default(),
            dir_couplers: DirCouplersParams::default(),
            glare: GlareParams::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintRenderingParams {
    pub density_curve_gamma: f64,
    pub glare: GlareParams,
}

impl Default for PrintRenderingParams {
    fn default() -> Self {
        Self { density_curve_gamma: 1.0, glare: GlareParams::default() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IOParams {
    pub input_color_space: String,
    pub input_cctf_decoding: bool,
    pub output_color_space: String,
    pub output_cctf_encoding: bool,
    pub crop: bool,
    pub crop_center: (f64, f64),
    pub crop_size: (f64, f64),
    pub upscale_factor: f64,
    pub scan_film: bool,
}

impl Default for IOParams {
    fn default() -> Self {
        Self {
            input_color_space: "ProPhoto RGB".into(),
            input_cctf_decoding: false,
            output_color_space: "sRGB".into(),
            output_cctf_encoding: true,
            crop: false,
            crop_center: (0.5, 0.5),
            crop_size: (0.1, 0.1),
            upscale_factor: 1.0,
            scan_film: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DebugMode { Off, Output, Inject, LutGeneration }

impl Default for DebugMode {
    fn default() -> Self { Self::Off }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DebugParams {
    pub deactivate_spatial_effects: bool,
    pub deactivate_stochastic_effects: bool,
    pub print_timings: bool,
    pub debug_mode: DebugMode,
    pub output_film_log_raw: bool,
    pub output_film_density_cmy: bool,
    pub output_print_density_cmy: bool,
    pub inject_film_density_cmy: bool,
}

impl Default for DebugParams {
    fn default() -> Self {
        Self {
            deactivate_spatial_effects: false,
            deactivate_stochastic_effects: false,
            print_timings: false,
            debug_mode: DebugMode::Off,
            output_film_log_raw: false,
            output_film_density_cmy: false,
            output_print_density_cmy: false,
            inject_film_density_cmy: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulationSettings {
    pub rgb_to_raw_method: String,
    pub bandpass_hanatos2025: bool,
    pub use_enlarger_lut: bool,
    pub use_scanner_lut: bool,
    pub lut_resolution: usize,
    pub use_fast_stats: bool,
    pub preview_max_size: usize,
    pub preview_mode: bool,
    pub neutral_print_filters_from_database: bool,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            rgb_to_raw_method: "hanatos2025".into(),
            bandpass_hanatos2025: true,
            use_enlarger_lut: false,
            use_scanner_lut: false,
            lut_resolution: 17,
            use_fast_stats: false,
            preview_max_size: 640,
            preview_mode: false,
            neutral_print_filters_from_database: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePhotoParams {
    pub film: Profile,
    pub print: Profile,
    #[serde(default)]
    pub film_render: FilmRenderingParams,
    #[serde(default)]
    pub print_render: PrintRenderingParams,
    #[serde(default)]
    pub camera: CameraParams,
    #[serde(default)]
    pub enlarger: EnlargerParams,
    #[serde(default)]
    pub scanner: ScannerParams,
    #[serde(default)]
    pub io: IOParams,
    #[serde(default)]
    pub debug: DebugParams,
    #[serde(default)]
    pub settings: SimulationSettings,
}

static NEUTRAL_PRINT_FILTERS: OnceLock<HashMap<String, HashMap<String, HashMap<String, [f64; 3]>>>> = OnceLock::new();

fn get_neutral_print_filters() -> &'static HashMap<String, HashMap<String, HashMap<String, [f64; 3]>>> {
    NEUTRAL_PRINT_FILTERS.get_or_init(|| {
        let json = include_str!("../../assets/data/filters/neutral_print_filters.json");
        serde_json::from_str(json).expect("Failed to parse neutral print filters database")
    })
}

impl RuntimePhotoParams {
    pub fn new(film: Profile, print: Profile) -> Self {
        Self {
            film,
            print,
            film_render: FilmRenderingParams::default(),
            print_render: PrintRenderingParams::default(),
            camera: CameraParams::default(),
            enlarger: EnlargerParams::default(),
            scanner: ScannerParams::default(),
            io: IOParams::default(),
            debug: DebugParams::default(),
            settings: SimulationSettings::default(),
        }
    }

    pub fn apply_database_neutral_print_filters(&mut self) {
        if !self.settings.neutral_print_filters_from_database {
            return;
        }

        let db = get_neutral_print_filters();
        let print_name = &self.print.info.name;
        let illuminant = &self.enlarger.illuminant;
        let film_name = &self.film.info.name;

        if let Some(illuminant_map) = db.get(print_name) {
            if let Some(film_map) = illuminant_map.get(illuminant) {
                if let Some(filters) = film_map.get(film_name) {
                    self.enlarger.c_filter_neutral = filters[0];
                    self.enlarger.m_filter_neutral = filters[1];
                    self.enlarger.y_filter_neutral = filters[2];
                }
            }
        }
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn apply_patch(&mut self, patch: RuntimeParamsPatch) {
        patch.apply_to(self);
    }

    pub fn apply_patch_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let patch = RuntimeParamsPatch::from_json(json)?;
        self.apply_patch(patch);
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeParamsPatch {
    pub camera: Option<CameraParamsPatch>,
    pub enlarger: Option<EnlargerParamsPatch>,
    pub scanner: Option<ScannerParamsPatch>,
    pub film_render: Option<FilmRenderingParamsPatch>,
    pub print_render: Option<PrintRenderingParamsPatch>,
    pub io: Option<IOParamsPatch>,
    pub debug: Option<DebugParamsPatch>,
    pub settings: Option<SimulationSettingsPatch>,
}

impl RuntimeParamsPatch {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn apply_to(self, params: &mut RuntimePhotoParams) {
        if let Some(patch) = self.camera { patch.apply_to(&mut params.camera); }
        if let Some(patch) = self.enlarger { patch.apply_to(&mut params.enlarger); }
        if let Some(patch) = self.scanner { patch.apply_to(&mut params.scanner); }
        if let Some(patch) = self.film_render { patch.apply_to(&mut params.film_render); }
        if let Some(patch) = self.print_render { patch.apply_to(&mut params.print_render); }
        if let Some(patch) = self.io { patch.apply_to(&mut params.io); }
        if let Some(patch) = self.debug { patch.apply_to(&mut params.debug); }
        if let Some(patch) = self.settings { patch.apply_to(&mut params.settings); }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraParamsPatch {
    pub exposure_compensation_ev: Option<f64>,
    pub auto_exposure: Option<bool>,
    pub auto_exposure_method: Option<String>,
    pub lens_blur_um: Option<f64>,
    pub film_format_mm: Option<f64>,
    pub filter_uv: Option<(f64, f64, f64)>,
    pub filter_ir: Option<(f64, f64, f64)>,
    pub diffusion_filter: Option<DiffusionFilterParams>,
}

impl CameraParamsPatch {
    pub fn apply_to(self, params: &mut CameraParams) {
        if let Some(v) = self.exposure_compensation_ev { params.exposure_compensation_ev = v; }
        if let Some(v) = self.auto_exposure { params.auto_exposure = v; }
        if let Some(v) = self.auto_exposure_method { params.auto_exposure_method = v; }
        if let Some(v) = self.lens_blur_um { params.lens_blur_um = v; }
        if let Some(v) = self.film_format_mm { params.film_format_mm = v; }
        if let Some(v) = self.filter_uv { params.filter_uv = v; }
        if let Some(v) = self.filter_ir { params.filter_ir = v; }
        if let Some(v) = self.diffusion_filter { params.diffusion_filter = v; }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EnlargerParamsPatch {
    pub illuminant: Option<String>,
    pub print_exposure: Option<f64>,
    pub print_exposure_compensation: Option<bool>,
    pub normalize_print_exposure: Option<bool>,
    pub y_filter_shift: Option<f64>,
    pub m_filter_shift: Option<f64>,
    pub y_filter_neutral: Option<f64>,
    pub m_filter_neutral: Option<f64>,
    pub c_filter_neutral: Option<f64>,
    pub lens_blur: Option<f64>,
    pub diffusion_filter: Option<DiffusionFilterParams>,
    pub preflash_exposure: Option<f64>,
    pub preflash_y_filter_shift: Option<f64>,
    pub preflash_m_filter_shift: Option<f64>,
}

impl EnlargerParamsPatch {
    pub fn apply_to(self, params: &mut EnlargerParams) {
        if let Some(v) = self.illuminant { params.illuminant = v; }
        if let Some(v) = self.print_exposure { params.print_exposure = v; }
        if let Some(v) = self.print_exposure_compensation { params.print_exposure_compensation = v; }
        if let Some(v) = self.normalize_print_exposure { params.normalize_print_exposure = v; }
        if let Some(v) = self.y_filter_shift { params.y_filter_shift = v; }
        if let Some(v) = self.m_filter_shift { params.m_filter_shift = v; }
        if let Some(v) = self.y_filter_neutral { params.y_filter_neutral = v; }
        if let Some(v) = self.m_filter_neutral { params.m_filter_neutral = v; }
        if let Some(v) = self.c_filter_neutral { params.c_filter_neutral = v; }
        if let Some(v) = self.lens_blur { params.lens_blur = v; }
        if let Some(v) = self.diffusion_filter { params.diffusion_filter = v; }
        if let Some(v) = self.preflash_exposure { params.preflash_exposure = v; }
        if let Some(v) = self.preflash_y_filter_shift { params.preflash_y_filter_shift = v; }
        if let Some(v) = self.preflash_m_filter_shift { params.preflash_m_filter_shift = v; }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScannerParamsPatch {
    pub lens_blur: Option<f64>,
    pub white_correction: Option<bool>,
    pub black_correction: Option<bool>,
    pub white_level: Option<f64>,
    pub black_level: Option<f64>,
    pub unsharp_mask: Option<(f64, f64)>,
}

impl ScannerParamsPatch {
    pub fn apply_to(self, params: &mut ScannerParams) {
        if let Some(v) = self.lens_blur { params.lens_blur = v; }
        if let Some(v) = self.white_correction { params.white_correction = v; }
        if let Some(v) = self.black_correction { params.black_correction = v; }
        if let Some(v) = self.white_level { params.white_level = v; }
        if let Some(v) = self.black_level { params.black_level = v; }
        if let Some(v) = self.unsharp_mask { params.unsharp_mask = v; }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FilmRenderingParamsPatch {
    pub density_curve_gamma: Option<f64>,
    pub grain: Option<GrainParams>,
    pub halation: Option<HalationParams>,
    pub dir_couplers: Option<DirCouplersParams>,
    pub glare: Option<GlareParams>,
}

impl FilmRenderingParamsPatch {
    pub fn apply_to(self, params: &mut FilmRenderingParams) {
        if let Some(v) = self.density_curve_gamma { params.density_curve_gamma = v; }
        if let Some(v) = self.grain { params.grain = v; }
        if let Some(v) = self.halation { params.halation = v; }
        if let Some(v) = self.dir_couplers { params.dir_couplers = v; }
        if let Some(v) = self.glare { params.glare = v; }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintRenderingParamsPatch {
    pub density_curve_gamma: Option<f64>,
    pub glare: Option<GlareParams>,
}

impl PrintRenderingParamsPatch {
    pub fn apply_to(self, params: &mut PrintRenderingParams) {
        if let Some(v) = self.density_curve_gamma { params.density_curve_gamma = v; }
        if let Some(v) = self.glare { params.glare = v; }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IOParamsPatch {
    pub input_color_space: Option<String>,
    pub input_cctf_decoding: Option<bool>,
    pub output_color_space: Option<String>,
    pub output_cctf_encoding: Option<bool>,
    pub crop: Option<bool>,
    pub crop_center: Option<(f64, f64)>,
    pub crop_size: Option<(f64, f64)>,
    pub upscale_factor: Option<f64>,
    pub scan_film: Option<bool>,
}

impl IOParamsPatch {
    pub fn apply_to(self, params: &mut IOParams) {
        if let Some(v) = self.input_color_space { params.input_color_space = v; }
        if let Some(v) = self.input_cctf_decoding { params.input_cctf_decoding = v; }
        if let Some(v) = self.output_color_space { params.output_color_space = v; }
        if let Some(v) = self.output_cctf_encoding { params.output_cctf_encoding = v; }
        if let Some(v) = self.crop { params.crop = v; }
        if let Some(v) = self.crop_center { params.crop_center = v; }
        if let Some(v) = self.crop_size { params.crop_size = v; }
        if let Some(v) = self.upscale_factor { params.upscale_factor = v; }
        if let Some(v) = self.scan_film { params.scan_film = v; }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DebugParamsPatch {
    pub deactivate_spatial_effects: Option<bool>,
    pub deactivate_stochastic_effects: Option<bool>,
    pub print_timings: Option<bool>,
    pub debug_mode: Option<DebugMode>,
    pub output_film_log_raw: Option<bool>,
    pub output_film_density_cmy: Option<bool>,
    pub output_print_density_cmy: Option<bool>,
    pub inject_film_density_cmy: Option<bool>,
}

impl DebugParamsPatch {
    pub fn apply_to(self, params: &mut DebugParams) {
        if let Some(v) = self.deactivate_spatial_effects { params.deactivate_spatial_effects = v; }
        if let Some(v) = self.deactivate_stochastic_effects { params.deactivate_stochastic_effects = v; }
        if let Some(v) = self.print_timings { params.print_timings = v; }
        if let Some(v) = self.debug_mode { params.debug_mode = v; }
        if let Some(v) = self.output_film_log_raw { params.output_film_log_raw = v; }
        if let Some(v) = self.output_film_density_cmy { params.output_film_density_cmy = v; }
        if let Some(v) = self.output_print_density_cmy { params.output_print_density_cmy = v; }
        if let Some(v) = self.inject_film_density_cmy { params.inject_film_density_cmy = v; }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulationSettingsPatch {
    pub rgb_to_raw_method: Option<String>,
    pub bandpass_hanatos2025: Option<bool>,
    pub use_enlarger_lut: Option<bool>,
    pub use_scanner_lut: Option<bool>,
    pub lut_resolution: Option<usize>,
    pub use_fast_stats: Option<bool>,
    pub preview_max_size: Option<usize>,
    pub preview_mode: Option<bool>,
    pub neutral_print_filters_from_database: Option<bool>,
}

impl SimulationSettingsPatch {
    pub fn apply_to(self, params: &mut SimulationSettings) {
        if let Some(v) = self.rgb_to_raw_method { params.rgb_to_raw_method = v; }
        if let Some(v) = self.bandpass_hanatos2025 { params.bandpass_hanatos2025 = v; }
        if let Some(v) = self.use_enlarger_lut { params.use_enlarger_lut = v; }
        if let Some(v) = self.use_scanner_lut { params.use_scanner_lut = v; }
        if let Some(v) = self.lut_resolution { params.lut_resolution = v; }
        if let Some(v) = self.use_fast_stats { params.use_fast_stats = v; }
        if let Some(v) = self.preview_max_size { params.preview_max_size = v; }
        if let Some(v) = self.preview_mode { params.preview_mode = v; }
        if let Some(v) = self.neutral_print_filters_from_database { params.neutral_print_filters_from_database = v; }
    }
}

fn patch_value<T, E>(value: &serde_json::Value, field: &str, target: &mut T) -> Result<(), E>
where
    T: DeserializeOwned,
    E: serde::de::Error,
{
    if let Some(v) = value.get(field) {
        *target = serde_json::from_value(v.clone()).map_err(E::custom)?;
    }
    Ok(())
}

macro_rules! impl_serde_from_default {
    ($ty:ty, [$($field:ident),+ $(,)?]) => {
        impl Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut state = serializer.serialize_struct(
                    stringify!($ty),
                    0 $(+ { let _ = stringify!($field); 1 })+
                )?;
                $(state.serialize_field(stringify!($field), &self.$field)?;)+
                state.end()
            }
        }

        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = serde_json::Value::deserialize(deserializer)?;
                let mut params = <$ty>::default();
                $(patch_value::<_, D::Error>(&value, stringify!($field), &mut params.$field)?;)+
                Ok(params)
            }
        }
    };
}

impl_serde_from_default!(DiffusionFilterParams, [
    active,
    filter_family,
    strength,
    spatial_scale,
    halo_warmth,
    core_intensity,
    core_size,
    halo_intensity,
    halo_size,
    bloom_intensity,
    bloom_size,
]);

impl_serde_from_default!(HalationParams, [
    active,
    scatter_amount,
    scatter_spatial_scale,
    halation_amount,
    halation_spatial_scale,
    scatter_core_um,
    scatter_tail_um,
    scatter_tail_weight,
    boost_ev,
    boost_range,
    protect_ev,
    halation_strength,
    halation_first_sigma_um,
    halation_n_bounces,
    halation_bounce_decay,
    halation_renormalize,
]);

impl_serde_from_default!(GrainParams, [
    active,
    sublayers_active,
    agx_particle_area_um2,
    agx_particle_scale,
    agx_particle_scale_layers,
    density_min,
    uniformity,
    blur,
    blur_dye_clouds_um,
    micro_structure,
    n_sub_layers,
]);

impl_serde_from_default!(DirCouplersParams, [
    active,
    amount,
    inhibition_samelayer,
    inhibition_interlayer,
    gamma_samelayer_rgb,
    gamma_interlayer_r_to_gb,
    gamma_interlayer_g_to_rb,
    gamma_interlayer_b_to_rg,
    diffusion_size_um,
]);

impl_serde_from_default!(GlareParams, [
    active,
    percent,
    roughness,
    blur,
]);
