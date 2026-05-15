//! RGB to raw exposure conversion.
//!
//! This module mirrors the Python `spektrafilm.utils.spectral_upsampling`
//! implementation as closely as possible without adding runtime dependencies:
//!
//! - RGB is converted through XYZ/xy and the Hanatos triangular/square LUT
//!   coordinate mapping.
//! - The bundled precomputed Hanatos 2025 half-float `.npy` spectra LUT is
//!   embedded and parsed directly when available.
//! - A coefficient `.lut` reader is kept as a secondary Hanatos path for builds
//!   that omit or replace the precomputed spectra table.
//! - A deterministic smooth spectral fallback remains available if no exact LUT
//!   path can be used.
//!
//! Remaining limitation: the Python Mallett 2019 path depends on the
//! `colour-science` package's tabulated Mallett basis functions. Those tables are
//! not bundled in the Rust crate, so the Rust function implements the same
//! RGB-basis integration shape and mid-gray normalisation with a compact smooth
//! basis approximation instead of the exact published table.



use crate::config::{CMFS, N_WL, WL_START, WL_STEP};
use crate::model::illuminants::standard_illuminant;

const HANATOS_SPECTRA_NPY: &[u8] = include_bytes!("../../assets/data/luts/spectral_upsampling/irradiance_xy_tc.npy");
const HANATOS_COEFFS_LUT: &[u8] = include_bytes!("../../assets/data/luts/spectral_upsampling/hanatos_irradiance_xy_coeffs_250304.lut");

const DEFAULT_SPECTRA_LUT_SIZE: usize = 192;
const EPS: f64 = 1.0e-10;

#[derive(Clone, Debug)]
pub struct RawLut2d {
    pub size_x: usize,
    pub size_y: usize,
    pub channels: usize,
    pub data: Vec<f64>,
}

#[derive(Clone, Copy)]
struct NpyArrayView<'a> {
    shape: [usize; 3],
    descr: &'a str,
    data: &'a [u8],
}

#[derive(Clone, Copy)]
struct CoeffLutView<'a> {
    width: usize,
    height: usize,
    data: &'a [u8],
}

#[inline]
fn srgb_decode(v: f64) -> f64 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

#[inline]
fn rec2020_decode(v: f64) -> f64 {
    // ITU-R BT.2020 inverse OETF. Negative values are handled symmetrically so
    // callers can pass slightly out-of-gamut working values without NaNs.
    let sign = if v < 0.0 { -1.0 } else { 1.0 };
    let a = v.abs();
    let linear = if a < 4.5 * 0.0181 { a / 4.5 } else { ((a + 0.0993) / 1.0993).powf(1.0 / 0.45) };
    sign * linear
}

#[inline]
fn prophoto_decode(v: f64) -> f64 {
    // ROMM/ProPhoto RGB uses a linear toe followed by gamma 1.8.
    let sign = if v < 0.0 { -1.0 } else { 1.0 };
    let a = v.abs();
    let linear = if a < 16.0 / 512.0 { a / 16.0 } else { a.powf(1.8) };
    sign * linear
}

#[inline]
fn decode_component(v: f64, color_space: &str, decode: bool) -> f64 {
    if !decode {
        return v;
    }
    match color_space {
        "ITU-R BT.2020" | "Rec2020" | "BT.2020" | "rec2020" | "bt2020" => rec2020_decode(v),
        "ProPhoto RGB" | "prophoto-rgb" | "ProPhoto" | "ROMM RGB" => prophoto_decode(v),
        _ => srgb_decode(v),
    }
}

fn rgb_space_to_xyz_matrix(color_space: &str) -> ([[f64; 3]; 3], [f64; 2]) {
    match color_space {
        "ProPhoto RGB" | "prophoto-rgb" | "ProPhoto" | "ROMM RGB" => (
            [
                [0.797_760_489_672_302_7, 0.135_185_837_175_740_31, 0.031_349_349_581_524_8],
                [0.288_071_128_229_293_4, 0.711_843_217_895_518_4, 0.000_085_653_960_606_915_38],
                [0.0, 0.0, 0.825_104_602_510_460_2],
            ],
            [0.345_67, 0.358_50], // D50
        ),
        "ITU-R BT.2020" | "Rec2020" | "BT.2020" | "rec2020" | "bt2020" => (
            [
                [0.636_958_048_3, 0.144_616_903_6, 0.168_880_975_2],
                [0.262_700_212_0, 0.677_998_071_5, 0.059_301_716_5],
                [0.0, 0.028_072_693_0, 1.060_985_057_7],
            ],
            [0.312_70, 0.329_00], // D65
        ),
        _ => (
            [
                [0.412_456_4, 0.357_576_1, 0.180_437_5],
                [0.212_672_9, 0.715_152_2, 0.072_175_0],
                [0.019_333_9, 0.119_192_0, 0.950_304_1],
            ],
            [0.312_70, 0.329_00], // D65
        ),
    }
}

fn mat3_mul_vec(m: [[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}



fn xy_to_xyz_white(xy: [f64; 2]) -> [f64; 3] {
    let y = xy[1].max(EPS);
    [xy[0] / y, 1.0, (1.0 - xy[0] - xy[1]) / y]
}

fn chromatic_adapt_cat02(xyz: [f64; 3], src_xy: [f64; 2], dst_xy: [f64; 2]) -> [f64; 3] {
    const CAT02: [[f64; 3]; 3] = [
        [0.7328, 0.4296, -0.1624],
        [-0.7036, 1.6975, 0.0061],
        [0.0030, 0.0136, 0.9834],
    ];
    const CAT02_INV: [[f64; 3]; 3] = [
        [1.096_123_820_835_514, -0.278_869_000_218_287, 0.182_745_179_382_773],
        [0.454_369_041_975_359, 0.473_533_154_307_412, 0.072_097_803_717_229],
        [-0.009_627_608_738_429, -0.005_698_031_216_113, 1.015_325_639_954_543],
    ];

    if (src_xy[0] - dst_xy[0]).abs() < 1e-8 && (src_xy[1] - dst_xy[1]).abs() < 1e-8 {
        return xyz;
    }

    let src_lms = mat3_mul_vec(CAT02, xy_to_xyz_white(src_xy));
    let dst_lms = mat3_mul_vec(CAT02, xy_to_xyz_white(dst_xy));
    let lms = mat3_mul_vec(CAT02, xyz);
    let adapted = [
        lms[0] * dst_lms[0] / src_lms[0].max(EPS),
        lms[1] * dst_lms[1] / src_lms[1].max(EPS),
        lms[2] * dst_lms[2] / src_lms[2].max(EPS),
    ];
    mat3_mul_vec(CAT02_INV, adapted)
}

fn rgb_to_xyz_native(rgb: [f64; 3], color_space: &str, decode: bool) -> ([f64; 3], [f64; 2]) {
    let linear_rgb = [
        decode_component(rgb[0], color_space, decode),
        decode_component(rgb[1], color_space, decode),
        decode_component(rgb[2], color_space, decode),
    ];
    let (m, source_white) = rgb_space_to_xyz_matrix(color_space);
    (mat3_mul_vec(m, linear_rgb), source_white)
}

fn rgb_to_xyz(rgb: [f64; 3], color_space: &str, decode: bool, reference_illuminant: &str) -> [f64; 3] {
    let (xyz, source_white) = rgb_to_xyz_native(rgb, color_space, decode);
    let target_white = illuminant_to_xy(reference_illuminant);
    chromatic_adapt_cat02(xyz, source_white, target_white)
}

fn rgb_to_linear_srgb(rgb: [f64; 3], color_space: &str, decode: bool) -> [f64; 3] {
    let (xyz, source_white) = rgb_to_xyz_native(rgb, color_space, decode);
    let xyz_d65 = chromatic_adapt_cat02(xyz, source_white, [0.312_70, 0.329_00]);
    const XYZ_TO_SRGB: [[f64; 3]; 3] = [
        [3.240_454_2, -1.537_138_5, -0.498_531_4],
        [-0.969_266_0, 1.876_010_8, 0.041_556_0],
        [0.055_643_4, -0.204_025_9, 1.057_225_2],
    ];
    let mut out = mat3_mul_vec(XYZ_TO_SRGB, xyz_d65);
    for v in &mut out {
        if !v.is_finite() || *v < 0.0 {
            *v = 0.0;
        }
    }
    out
}

/// Converts triangular xy-like chromaticity coordinates into square LUT
/// coordinates. This is Python `_tri2quad` and is used before indexing the
/// Hanatos TC LUT.
pub fn tri2quad(tc: [f64; 2]) -> [f64; 2] {
    let tx = tc[0];
    let ty = tc[1];
    let denom = (1.0 - tx).max(EPS);
    let x = ((1.0 - tx) * (1.0 - tx)).clamp(0.0, 1.0);
    let y = (ty / denom).clamp(0.0, 1.0);
    [x, y]
}

/// Converts square Hanatos LUT coordinates back into triangular xy-like
/// coordinates. This is Python `_quad2tri`.
pub fn quad2tri(xy: [f64; 2]) -> [f64; 2] {
    let x = xy[0].clamp(0.0, 1.0).sqrt();
    let y = xy[1].clamp(0.0, 1.0);
    [1.0 - x, y * x]
}

/// Compute xy chromaticity of a bundled/reference illuminant using the standard
/// observer colour matching functions.
pub fn illuminant_to_xy(illuminant_label: &str) -> [f64; 2] {
    let illu = standard_illuminant(illuminant_label);
    let mut xyz = [0.0; 3];
    for wl in 0..N_WL {
        xyz[0] += illu[wl] * CMFS[wl][0];
        xyz[1] += illu[wl] * CMFS[wl][1];
        xyz[2] += illu[wl] * CMFS[wl][2];
    }
    let sum = xyz[0] + xyz[1] + xyz[2];
    if sum > EPS && sum.is_finite() { [xyz[0] / sum, xyz[1] / sum] } else { [0.3127, 0.3290] }
}

/// Convert a flat RGB buffer to Hanatos square TC coordinates plus brightness
/// scale `b = X + Y + Z`. Returns `(tc_flat, b)`, where `tc_flat` has shape
/// `[N × 2]` and `b` has shape `[N]`.
pub fn rgb_to_tc_b(
    rgb: &[f64],
    color_space: &str,
    apply_cctf_decoding: bool,
    reference_illuminant: &str,
) -> (Vec<f64>, Vec<f64>) {
    let n_px = rgb.len() / 3;
    let mut tc = vec![0.0; n_px * 2];
    let mut b = vec![0.0; n_px];

    for px in 0..n_px {
        let xyz = rgb_to_xyz(
            [rgb[px * 3], rgb[px * 3 + 1], rgb[px * 3 + 2]],
            color_space,
            apply_cctf_decoding,
            reference_illuminant,
        );
        let sum = xyz[0] + xyz[1] + xyz[2];
        let safe_sum = sum.max(EPS);
        let xy = [(xyz[0] / safe_sum).clamp(0.0, 1.0), (xyz[1] / safe_sum).clamp(0.0, 1.0)];
        let p_tc = tri2quad(xy);
        tc[px * 2] = p_tc[0];
        tc[px * 2 + 1] = p_tc[1];
        b[px] = if sum.is_finite() { sum.max(0.0) } else { 0.0 };
    }

    (tc, b)
}

fn parse_npy_array(bytes: &[u8]) -> Option<NpyArrayView<'_>> {
    if bytes.len() < 16 || &bytes[0..6] != b"\x93NUMPY" {
        return None;
    }
    let major = bytes[6];
    let header_len_offset = 8usize;
    let (header_len, data_offset) = match major {
        1 => {
            let len = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
            (len, 10usize)
        }
        2 | 3 => {
            if bytes.len() < 12 {
                return None;
            }
            let len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
            (len, 12usize)
        }
        _ => return None,
    };
    let header_start = header_len_offset + if major == 1 { 2 } else { 4 };
    let header_end = header_start.checked_add(header_len)?;
    if header_end > bytes.len() || data_offset != header_start {
        return None;
    }
    let header = std::str::from_utf8(&bytes[header_start..header_end]).ok()?;
    if !header.contains("'fortran_order': False") && !header.contains("\"fortran_order\": False") {
        return None;
    }
    let descr = if header.contains("'<f2'") || header.contains("\"<f2\"") {
        "<f2"
    } else if header.contains("'<f4'") || header.contains("\"<f4\"") {
        "<f4"
    } else if header.contains("'<f8'") || header.contains("\"<f8\"") {
        "<f8"
    } else {
        return None;
    };

    let shape_key = "'shape':";
    let shape_pos = header.find(shape_key).or_else(|| header.find("\"shape\":"))?;
    let after_shape = &header[shape_pos..];
    let open = after_shape.find('(')?;
    let close = after_shape[open + 1..].find(')')? + open + 1;
    let mut dims = [0usize; 3];
    let mut n_dims = 0usize;
    for part in after_shape[open + 1..close].split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        if n_dims >= 3 {
            return None;
        }
        dims[n_dims] = p.parse::<usize>().ok()?;
        n_dims += 1;
    }
    if n_dims != 3 || dims.iter().any(|d| *d == 0) {
        return None;
    }
    let bytes_per = match descr {
        "<f2" => 2,
        "<f4" => 4,
        "<f8" => 8,
        _ => return None,
    };
    let expected = dims[0].checked_mul(dims[1])?.checked_mul(dims[2])?.checked_mul(bytes_per)?;
    let data = bytes.get(header_end..header_end.checked_add(expected)?)?;
    Some(NpyArrayView { shape: dims, descr, data })
}

#[inline]
fn f16_to_f64(bits: u16) -> f64 {
    let sign = if (bits & 0x8000) != 0 { -1.0 } else { 1.0 };
    let exp = ((bits >> 10) & 0x1f) as i32;
    let frac = (bits & 0x03ff) as u32;
    match exp {
        0 => {
            if frac == 0 { sign * 0.0 } else { sign * (frac as f64 / 1024.0) * 2f64.powi(-14) }
        }
        31 => {
            if frac == 0 { sign * f64::INFINITY } else { f64::NAN }
        }
        _ => sign * (1.0 + frac as f64 / 1024.0) * 2f64.powi(exp - 15),
    }
}

fn npy_value(view: NpyArrayView<'_>, index: usize) -> f64 {
    match view.descr {
        "<f2" => {
            let o = index * 2;
            f16_to_f64(u16::from_le_bytes([view.data[o], view.data[o + 1]]))
        }
        "<f4" => {
            let o = index * 4;
            f32::from_le_bytes([view.data[o], view.data[o + 1], view.data[o + 2], view.data[o + 3]]) as f64
        }
        "<f8" => {
            let o = index * 8;
            f64::from_le_bytes([
                view.data[o],
                view.data[o + 1],
                view.data[o + 2],
                view.data[o + 3],
                view.data[o + 4],
                view.data[o + 5],
                view.data[o + 6],
                view.data[o + 7],
            ])
        }
        _ => 0.0,
    }
}

fn parse_coeff_lut(bytes: &[u8]) -> Option<CoeffLutView<'_>> {
    if bytes.len() < 16 {
        return None;
    }
    let width = i32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let height = i32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
    if width == 0 || height == 0 {
        return None;
    }
    let expected = width.checked_mul(height)?.checked_mul(4)?.checked_mul(4)?;
    let data = bytes.get(16..16 + expected)?;
    Some(CoeffLutView { width, height, data })
}

fn coeff_value(view: CoeffLutView<'_>, x: usize, y: usize, channel: usize) -> f64 {
    let x = x.min(view.width - 1);
    let y = y.min(view.height - 1);
    let channel = channel.min(3);
    let o = ((y * view.width + x) * 4 + channel) * 4;
    f32::from_le_bytes([view.data[o], view.data[o + 1], view.data[o + 2], view.data[o + 3]]) as f64
}

#[inline]
fn clamp_coordinate(coord: f64, len: usize) -> f64 {
    if len <= 1 {
        return 0.0;
    }
    coord.clamp(0.0, (len - 1) as f64)
}

#[inline]
fn cubic_base_fraction(coord: f64, len: usize) -> (isize, f64) {
    let coord = clamp_coordinate(coord, len);
    if coord >= (len - 1) as f64 {
        return ((len - 2) as isize, 1.0);
    }
    let base = coord.floor() as isize;
    (base, coord - base as f64)
}

#[inline]
fn safe_index(idx: isize, len: usize) -> usize {
    if len <= 1 {
        0
    } else if idx < 0 {
        (-idx).min((len - 1) as isize) as usize
    } else if idx >= len as isize {
        (2 * (len as isize - 1) - idx).clamp(0, len as isize - 1) as usize
    } else {
        idx as usize
    }
}

#[inline]
fn mitchell_weight(t: f64) -> f64 {
    let b = 1.0 / 3.0;
    let c = 1.0 / 3.0;
    let x = t.abs();
    if x < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * x.powi(3) + (-18.0 + 12.0 * b + 6.0 * c) * x.powi(2) + (6.0 - 2.0 * b)) / 6.0
    } else if x < 2.0 {
        ((-b - 6.0 * c) * x.powi(3) + (6.0 * b + 30.0 * c) * x.powi(2) + (-12.0 * b - 48.0 * c) * x + (8.0 * b + 24.0 * c)) / 6.0
    } else {
        0.0
    }
}

fn interpolate_raw_lut(lut: &RawLut2d, xy: [f64; 2]) -> [f64; 3] {
    if lut.size_x == 0 || lut.size_y == 0 || lut.channels < 3 {
        return [0.0; 3];
    }
    if lut.size_x < 2 || lut.size_y < 2 {
        return [lut.data[0], lut.data[1], lut.data[2]];
    }

    let x = xy[0].clamp(0.0, 1.0) * (lut.size_x - 1) as f64;
    let y = xy[1].clamp(0.0, 1.0) * (lut.size_y - 1) as f64;
    let (xb, xf) = cubic_base_fraction(x, lut.size_x);
    let (yb, yf) = cubic_base_fraction(y, lut.size_y);
    let wx = [mitchell_weight(xf + 1.0), mitchell_weight(xf), mitchell_weight(xf - 1.0), mitchell_weight(xf - 2.0)];
    let wy = [mitchell_weight(yf + 1.0), mitchell_weight(yf), mitchell_weight(yf - 1.0), mitchell_weight(yf - 2.0)];

    let mut out = [0.0; 3];
    let mut weight_sum = 0.0;
    for i in 0..4 {
        let xi = safe_index(xb - 1 + i as isize, lut.size_x);
        for j in 0..4 {
            let yj = safe_index(yb - 1 + j as isize, lut.size_y);
            let w = wx[i] * wy[j];
            weight_sum += w;
            let base = (xi * lut.size_y + yj) * lut.channels;
            for ch in 0..3 {
                out[ch] += w * lut.data[base + ch];
            }
        }
    }
    if weight_sum.abs() > EPS {
        for v in &mut out {
            *v /= weight_sum;
        }
    }
    out
}

fn interpolate_coeffs(view: CoeffLutView<'_>, tc: [f64; 2]) -> [f64; 4] {
    let x = tc[0].clamp(0.0, 1.0) * (view.width - 1) as f64;
    let y = tc[1].clamp(0.0, 1.0) * (view.height - 1) as f64;
    let (xb, xf) = cubic_base_fraction(x, view.width);
    let (yb, yf) = cubic_base_fraction(y, view.height);
    let wx = [mitchell_weight(xf + 1.0), mitchell_weight(xf), mitchell_weight(xf - 1.0), mitchell_weight(xf - 2.0)];
    let wy = [mitchell_weight(yf + 1.0), mitchell_weight(yf), mitchell_weight(yf - 1.0), mitchell_weight(yf - 2.0)];

    let mut out = [0.0; 4];
    let mut weight_sum = 0.0;
    for i in 0..4 {
        let xi = safe_index(xb - 1 + i as isize, view.width);
        for j in 0..4 {
            let yj = safe_index(yb - 1 + j as isize, view.height);
            let w = wx[i] * wy[j];
            weight_sum += w;
            for ch in 0..4 {
                out[ch] += w * coeff_value(view, xi, yj, ch);
            }
        }
    }
    if weight_sum.abs() > EPS {
        for v in &mut out {
            *v /= weight_sum;
        }
    }
    out
}

/// Build the Hanatos 2025 square-TC raw LUT for a camera sensitivity.
pub fn compute_hanatos2025_tc_lut(sensitivity: &[f64]) -> Option<RawLut2d> {
    let npy = parse_npy_array(HANATOS_SPECTRA_NPY)?;
    let size_x = npy.shape[0];
    let size_y = npy.shape[1];
    let n_wl = npy.shape[2].min(N_WL).min(sensitivity.len() / 3);
    if n_wl == 0 {
        return None;
    }
    let mut data = vec![0.0; size_x * size_y * 3];
    for i in 0..size_x {
        for j in 0..size_y {
            let out_base = (i * size_y + j) * 3;
            for wl in 0..n_wl {
                let spec_index = (i * size_y + j) * npy.shape[2] + wl;
                let s = npy_value(npy, spec_index);
                if !s.is_finite() {
                    continue;
                }
                for ch in 0..3 {
                    let sens = sensitivity[wl * 3 + ch];
                    if sens.is_finite() {
                        data[out_base + ch] += s * sens;
                    }
                }
            }
        }
    }
    Some(RawLut2d { size_x, size_y, channels: 3, data })
}

fn coeff_spectrum_raw(tc: [f64; 2], b: f64, sensitivity: &[f64]) -> Option<[f64; 3]> {
    let coeff_lut = parse_coeff_lut(HANATOS_COEFFS_LUT)?;
    let coeffs = interpolate_coeffs(coeff_lut, tc);
    let denom = coeffs[3];
    if !denom.is_finite() || denom.abs() <= EPS {
        return None;
    }
    let n_wl = N_WL.min(sensitivity.len() / 3);
    let mut raw = [0.0; 3];
    for wl_i in 0..n_wl {
        let wl = WL_START + wl_i as f64 * WL_STEP;
        let x = (coeffs[0] * wl + coeffs[1]) * wl + coeffs[2];
        let y = 1.0 / (x * x + 1.0).sqrt();
        let spectrum = ((0.5 * x * y + 0.5) / denom).max(0.0) * b;
        if !spectrum.is_finite() {
            continue;
        }
        for ch in 0..3 {
            let sens = sensitivity[wl_i * 3 + ch];
            if sens.is_finite() {
                raw[ch] += spectrum * sens;
            }
        }
    }
    Some(raw)
}

/// Smooth spectrum reconstruction from RGB/XYZ chromaticity. This is the final
/// safety net when neither bundled Hanatos LUT path can be used.
fn reconstruct_spectrum(xyz: [f64; 3], reference_illuminant: &str) -> [f64; N_WL] {
    let y = xyz[1].max(0.0);
    let sum = (xyz[0] + xyz[1] + xyz[2]).max(1e-12);
    let x_chroma = (xyz[0] / sum).clamp(0.0, 1.0);
    let y_chroma = (xyz[1] / sum).clamp(0.0, 1.0);
    let illuminant = standard_illuminant(reference_illuminant);

    let red_w = (x_chroma / 0.64).clamp(0.0, 2.0);
    let green_w = (y_chroma / 0.33).clamp(0.0, 2.0);
    let blue_w = ((1.0 - x_chroma - y_chroma).max(0.0) / 0.06).clamp(0.0, 2.0);
    let mut spec = [0.0; N_WL];
    for i in 0..N_WL {
        let wl = WL_START + i as f64 * WL_STEP;
        let r = red_w * (-0.5 * ((wl - 610.0) / 55.0).powi(2)).exp();
        let g = green_w * (-0.5 * ((wl - 540.0) / 45.0).powi(2)).exp();
        let b = blue_w * (-0.5 * ((wl - 455.0) / 35.0).powi(2)).exp();
        spec[i] = (r + g + b).max(0.0) * illuminant[i];
    }
    let y_norm: f64 = spec.iter().zip(CMFS.iter()).map(|(s, cmf)| s * cmf[1]).sum();
    let scale = if y_norm > 1e-12 { y / y_norm } else { 0.0 };
    for v in &mut spec { *v *= scale; }
    spec
}

fn raw_from_smooth_fallback(
    rgb: [f64; 3],
    sensitivity: &[f64],
    color_space: &str,
    apply_cctf_decoding: bool,
    reference_illuminant: &str,
) -> [f64; 3] {
    let xyz = rgb_to_xyz(rgb, color_space, apply_cctf_decoding, reference_illuminant);
    let spec = reconstruct_spectrum(xyz, reference_illuminant);
    let mut out = [0.0; 3];
    for ch in 0..3 {
        for wl in 0..N_WL.min(sensitivity.len() / 3) {
            let s = sensitivity[wl * 3 + ch];
            if s.is_finite() { out[ch] += spec[wl] * s; }
        }
        out[ch] = out[ch].max(0.0);
    }
    out
}

/// Convert RGB to raw film exposure using the Hanatos 2025 LUT flow.
///
/// The preferred path is: RGB → adapted XYZ → xy/b → square TC → embedded
/// spectra LUT integrated with the supplied sensitivity → cubic 2-D lookup →
/// scale by `b`. If the `.npy` asset cannot be parsed, this falls back to the
/// coefficient `.lut`; if that also fails, the smooth Gaussian fallback is used.
pub fn rgb_to_raw_hanatos2025(
    rgb: &[f64],
    sensitivity: &[f64],
    color_space: &str,
    apply_cctf_decoding: bool,
    reference_illuminant: &str,
    raw_lut: Option<&RawLut2d>,
) -> Vec<f64> {
    let n_px = rgb.len() / 3;
    let mut out = vec![0.0; n_px * 3];
    if n_px == 0 || sensitivity.len() < 3 {
        return out;
    }

    let (tc, b) = rgb_to_tc_b(rgb, color_space, apply_cctf_decoding, reference_illuminant);

    for px in 0..n_px {
        let raw = if let Some(lut) = raw_lut.as_ref() {
            let mut r = interpolate_raw_lut(lut, [tc[px * 2], tc[px * 2 + 1]]);
            for v in &mut r {
                *v *= b[px];
            }
            r
        } else if let Some(r) = coeff_spectrum_raw([tc[px * 2], tc[px * 2 + 1]], b[px], sensitivity) {
            r
        } else {
            raw_from_smooth_fallback(
                [rgb[px * 3], rgb[px * 3 + 1], rgb[px * 3 + 2]],
                sensitivity,
                color_space,
                apply_cctf_decoding,
                reference_illuminant,
            )
        };
        out[px * 3] = raw[0].max(0.0);
        out[px * 3 + 1] = raw[1].max(0.0);
        out[px * 3 + 2] = raw[2].max(0.0);
    }
    out
}

fn mallett_basis_value(channel: usize, wl: f64) -> f64 {
    // Compact non-negative approximation to the sRGB Mallett basis functions.
    // It preserves the broad spectral support and RGB-basis integration API, but
    // is not the exact colour-science table (see module-level limitation).
    match channel {
        0 => {
            1.00 * (-0.5 * ((wl - 610.0) / 45.0).powi(2)).exp()
                + 0.18 * (-0.5 * ((wl - 545.0) / 75.0).powi(2)).exp()
        }
        1 => {
            0.92 * (-0.5 * ((wl - 535.0) / 42.0).powi(2)).exp()
                + 0.12 * (-0.5 * ((wl - 610.0) / 80.0).powi(2)).exp()
                + 0.08 * (-0.5 * ((wl - 455.0) / 55.0).powi(2)).exp()
        }
        _ => {
            1.05 * (-0.5 * ((wl - 455.0) / 32.0).powi(2)).exp()
                + 0.12 * (-0.5 * ((wl - 500.0) / 80.0).powi(2)).exp()
        }
    }
}

/// Convert RGB to raw film exposure using a Mallett-2019-style RGB spectral
/// basis integration and the Python path's green-channel mid-gray normalisation.
pub fn rgb_to_raw_mallett2019(
    rgb: &[f64],
    sensitivity: &[f64],
    color_space: &str,
    apply_cctf_decoding: bool,
    reference_illuminant: &str,
) -> Vec<f64> {
    let n_px = rgb.len() / 3;
    let mut out = vec![0.0; n_px * 3];
    if n_px == 0 || sensitivity.len() < 3 {
        return out;
    }

    let illuminant = standard_illuminant(reference_illuminant);
    let n_wl = N_WL.min(sensitivity.len() / 3);
    let mut basis_with_illuminant = [[0.0; 3]; N_WL];
    for wl_i in 0..n_wl {
        let wl = WL_START + wl_i as f64 * WL_STEP;
        for basis_ch in 0..3 {
            basis_with_illuminant[wl_i][basis_ch] = mallett_basis_value(basis_ch, wl) * illuminant[wl_i];
        }
    }

    let raw_midgray_green: f64 = (0..n_wl)
        .map(|wl_i| illuminant[wl_i] * 0.184 * sensitivity[wl_i * 3 + 1])
        .filter(|v| v.is_finite())
        .sum::<f64>()
        .max(EPS);

    for px in 0..n_px {
        let lrgb = rgb_to_linear_srgb(
            [rgb[px * 3], rgb[px * 3 + 1], rgb[px * 3 + 2]],
            color_space,
            apply_cctf_decoding,
        );
        for raw_ch in 0..3 {
            let mut raw = 0.0;
            for wl_i in 0..n_wl {
                let sens = sensitivity[wl_i * 3 + raw_ch];
                if !sens.is_finite() {
                    continue;
                }
                let spectrum = lrgb[0] * basis_with_illuminant[wl_i][0]
                    + lrgb[1] * basis_with_illuminant[wl_i][1]
                    + lrgb[2] * basis_with_illuminant[wl_i][2];
                raw += spectrum.max(0.0) * sens;
            }
            out[px * 3 + raw_ch] = (raw / raw_midgray_green).max(0.0);
        }
    }
    out
}

/// Direct interpolation of the embedded Hanatos spectra LUT for callers that
/// need a smooth spectrum for a single RGB sample. The output is a flat spectrum
/// of length `N_WL`. Falls back to the smooth reconstruction when the embedded
/// LUT cannot be parsed.
pub fn rgb_to_smooth_spectrum(
    rgb: [f64; 3],
    color_space: &str,
    apply_cctf_decoding: bool,
    reference_illuminant: &str,
) -> Vec<f64> {
    let flat_rgb = [rgb[0], rgb[1], rgb[2]];
    let (tc, b) = rgb_to_tc_b(&flat_rgb, color_space, apply_cctf_decoding, reference_illuminant);
    if let Some(npy) = parse_npy_array(HANATOS_SPECTRA_NPY) {
        let size_x = npy.shape[0];
        let size_y = npy.shape[1];
        let n_wl = npy.shape[2].min(N_WL);
        let mut spectrum = vec![0.0; N_WL];
        for wl in 0..n_wl {
            let mut lut = RawLut2d { size_x, size_y, channels: 1, data: vec![0.0; size_x * size_y] };
            for i in 0..size_x {
                for j in 0..size_y {
                    lut.data[i * size_y + j] = npy_value(npy, (i * size_y + j) * npy.shape[2] + wl);
                }
            }
            let v = interpolate_scalar_lut(&lut, [tc[0], tc[1]]);
            spectrum[wl] = v.max(0.0) * b[0];
        }
        return spectrum;
    }

    reconstruct_spectrum(
        rgb_to_xyz(rgb, color_space, apply_cctf_decoding, reference_illuminant),
        reference_illuminant,
    )
    .to_vec()
}

fn interpolate_scalar_lut(lut: &RawLut2d, xy: [f64; 2]) -> f64 {
    if lut.size_x == 0 || lut.size_y == 0 || lut.data.is_empty() {
        return 0.0;
    }
    if lut.size_x < 2 || lut.size_y < 2 {
        return lut.data[0];
    }
    let x = xy[0].clamp(0.0, 1.0) * (lut.size_x - 1) as f64;
    let y = xy[1].clamp(0.0, 1.0) * (lut.size_y - 1) as f64;
    let (xb, xf) = cubic_base_fraction(x, lut.size_x);
    let (yb, yf) = cubic_base_fraction(y, lut.size_y);
    let wx = [mitchell_weight(xf + 1.0), mitchell_weight(xf), mitchell_weight(xf - 1.0), mitchell_weight(xf - 2.0)];
    let wy = [mitchell_weight(yf + 1.0), mitchell_weight(yf), mitchell_weight(yf - 1.0), mitchell_weight(yf - 2.0)];
    let mut out = 0.0;
    let mut weight_sum = 0.0;
    for i in 0..4 {
        let xi = safe_index(xb - 1 + i as isize, lut.size_x);
        for j in 0..4 {
            let yj = safe_index(yb - 1 + j as isize, lut.size_y);
            let w = wx[i] * wy[j];
            weight_sum += w;
            out += w * lut.data[xi * lut.size_y + yj];
        }
    }
    if weight_sum.abs() > EPS { out / weight_sum } else { out }
}

#[allow(dead_code)]
fn _embedded_spectra_lut_size_hint() -> usize { DEFAULT_SPECTRA_LUT_SIZE }
