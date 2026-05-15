//! Colour and density conversion utilities.

use crate::config::N_WL;

/// CIE D65 white point chromaticity used by sRGB, Display P3 and BT.2020.
pub const D65_WHITEPOINT_XY: [f64; 2] = [0.3127, 0.3290];
/// CIE D50 white point chromaticity used by ProPhoto RGB.
pub const D50_WHITEPOINT_XY: [f64; 2] = [0.34567, 0.35850];
/// ACES white point chromaticity used by ACES2065-1 / AP0.
pub const ACES_WHITEPOINT_XY: [f64; 2] = [0.32168, 0.33767];

const IDENTITY_3: [[f64; 3]; 3] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];

/// Supported RGB colour spaces for matrix and CCTF conversions.
///
/// Matrix values are expressed relative to each colour space reference white,
/// matching how `colour-science` exposes `RGB_COLOURSPACES` before optional
/// chromatic adaptation is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbColorSpace {
    Srgb,
    ProPhotoRgb,
    Rec2020,
    DisplayP3,
    Aces2065_1,
}

/// Chromatic adaptation transforms available for XYZ conversion helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChromaticAdaptationTransform {
    Bradford,
    Cat02,
}

/// Static RGB colour space definition.
#[derive(Clone, Copy, Debug)]
pub struct RgbColorSpaceSpec {
    pub name: &'static str,
    pub whitepoint_xy: [f64; 2],
    pub rgb_to_xyz: [[f64; 3]; 3],
    pub xyz_to_rgb: [[f64; 3]; 3],
}

impl RgbColorSpace {
    /// Return the matrix and white point definition for this colour space.
    pub const fn spec(self) -> RgbColorSpaceSpec {
        match self {
            RgbColorSpace::Srgb => RgbColorSpaceSpec {
                name: "sRGB",
                whitepoint_xy: D65_WHITEPOINT_XY,
                rgb_to_xyz: [
                    [0.4124564, 0.3575761, 0.1804375],
                    [0.2126729, 0.7151522, 0.0721750],
                    [0.0193339, 0.1191920, 0.9503041],
                ],
                xyz_to_rgb: [
                    [3.2404542, -1.5371385, -0.4985314],
                    [-0.9692660, 1.8760108, 0.0415560],
                    [0.0556434, -0.2040259, 1.0572252],
                ],
            },
            RgbColorSpace::ProPhotoRgb => RgbColorSpaceSpec {
                name: "ProPhoto RGB",
                whitepoint_xy: D50_WHITEPOINT_XY,
                rgb_to_xyz: [
                    [0.7977604896723027, 0.13518583717574031, 0.0313493495815248],
                    [0.2880711282292934, 0.7118432178955184, 0.00008565396060691538],
                    [0.0, 0.0, 0.8251046025104602],
                ],
                xyz_to_rgb: [
                    [1.3459433, -0.2556075, -0.0511118],
                    [-0.5445989, 1.5081673, 0.0205351],
                    [0.0, 0.0, 1.2118128],
                ],
            },
            RgbColorSpace::Rec2020 => RgbColorSpaceSpec {
                name: "ITU-R BT.2020",
                whitepoint_xy: D65_WHITEPOINT_XY,
                rgb_to_xyz: [
                    [0.6369580483012914, 0.14461690358620832, 0.1688809751641721],
                    [0.2627002120112671, 0.6779980715188708, 0.05930171646986196],
                    [0.0, 0.028072693049087428, 1.060985057710791],
                ],
                xyz_to_rgb: [
                    [1.716651187971268, -0.355670783776393, -0.25336628137366],
                    [-0.666684351832489, 1.616481236634939, 0.0157685458139111],
                    [0.017639857445311, -0.042770613257809, 0.942103121235474],
                ],
            },
            RgbColorSpace::DisplayP3 => RgbColorSpaceSpec {
                name: "Display P3",
                whitepoint_xy: D65_WHITEPOINT_XY,
                rgb_to_xyz: [
                    [0.4865709486482162, 0.26566769316909306, 0.1982172852343625],
                    [0.2289745640697488, 0.6917385218365064, 0.079286914093745],
                    [0.0, 0.04511338185890264, 1.0439443689009755],
                ],
                xyz_to_rgb: [
                    [2.493496911941425, -0.931383617919124, -0.402710784450717],
                    [-0.829488969561575, 1.762664060318346, 0.023624685841943],
                    [0.035845830243784, -0.076172389268041, 0.956884524007687],
                ],
            },
            RgbColorSpace::Aces2065_1 => RgbColorSpaceSpec {
                name: "ACES2065-1",
                whitepoint_xy: ACES_WHITEPOINT_XY,
                rgb_to_xyz: [
                    [0.9525523959381858, 0.0, 0.000093678631681925],
                    [0.343966449765075, 0.728166096613485, -0.07213254637856],
                    [0.0, 0.0, 1.0088251843515865],
                ],
                xyz_to_rgb: [
                    [1.049811017497974, 0.0, -0.000097484540883],
                    [-0.495903023077648, 1.373313045815535, 0.098240036083271],
                    [0.0, 0.0, 0.991252018200499],
                ],
            },
        }
    }
}

/// Parse common `colour-science` colour space names and aliases.
pub fn parse_rgb_color_space(name: &str) -> Option<RgbColorSpace> {
    let normalized: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect();

    match normalized.as_str() {
        "srgb" => Some(RgbColorSpace::Srgb),
        "prophoto" | "prophotorgb" | "romm" | "rommrgb" => Some(RgbColorSpace::ProPhotoRgb),
        "rec2020" | "bt2020" | "iturbt2020" | "itur2020" | "rec2100" => Some(RgbColorSpace::Rec2020),
        "displayp3" | "p3" | "dcip3d65" => Some(RgbColorSpace::DisplayP3),
        "aces20651" | "aces2065" | "aces" | "acesap0" => Some(RgbColorSpace::Aces2065_1),
        _ => None,
    }
}

/// Convert spectral density to transmitted light.
///
/// `density_spectral` : flat [N_PX × N_WL].
/// `illuminant` : [N_WL].
/// Returns flat [N_PX × N_WL].
pub fn density_to_light(density_spectral: &[f64], illuminant: &[f64; N_WL], n_wl: usize) -> Vec<f64> {
    let n_px = density_spectral.len() / n_wl;
    let mut out = vec![0.0f64; n_px * n_wl];
    for px in 0..n_px {
        for wl in 0..n_wl {
            let d = density_spectral[px * n_wl + wl];
            // transmitted = 10^(-d) * illuminant
            let t = if d.is_finite() { 10.0_f64.powf(-d) } else { 0.0 };
            out[px * n_wl + wl] = t * illuminant[wl];
        }
    }
    out
}

/// Integrate light against colour matching functions to produce XYZ.
///
/// `light` : flat [N_PX × N_WL].
/// `cmfs` : [[f64;3]; N_WL] — CIE 1931 CMFs.
/// Returns flat [N_PX × 3].
pub fn light_to_xyz(light: &[f64], cmfs: &[[f64; 3]; N_WL], normalization: f64, n_wl: usize) -> Vec<f64> {
    let n_px = light.len() / n_wl;
    let mut out = vec![0.0f64; n_px * 3];
    for px in 0..n_px {
        let mut xyz = [0.0f64; 3];
        for wl in 0..n_wl {
            let l = light[px * n_wl + wl];
            if l.is_finite() {
                for ch in 0..3 { xyz[ch] += l * cmfs[wl][ch]; }
            }
        }
        let norm = if normalization > 1e-30 { normalization } else { 1.0 };
        for ch in 0..3 { out[px * 3 + ch] = xyz[ch] / norm; }
    }
    out
}

/// Convert XYZ to linear sRGB via the standard 3×3 matrix (D65 adapted).
pub fn xyz_to_srgb_linear(xyz: &[f64]) -> Vec<f64> {
    apply_matrix_to_triples(xyz, RgbColorSpace::Srgb.spec().xyz_to_rgb)
}

/// Apply sRGB gamma encoding (piecewise linear + power curve).
///
/// This function keeps the previous behaviour of clamping to display range before
/// encoding. Use [`cctf_encode`] for non-clamping, colour-space-aware encoding.
pub fn srgb_gamma_encode(linear: &[f64]) -> Vec<f64> {
    linear.iter().map(|&v| {
        let v = v.clamp(0.0, 1.0);
        srgb_encode_value(v)
    }).collect()
}

/// Decode sRGB gamma.
pub fn srgb_gamma_decode(encoded: &[f64]) -> Vec<f64> {
    encoded.iter().map(|&v| srgb_decode_value(v)).collect()
}

/// Convert XYZ to xy chromaticity.
pub fn xyz_to_xy(xyz: &[f64; 3]) -> [f64; 2] {
    let sum = xyz[0] + xyz[1] + xyz[2];
    if sum < 1e-30 { return D65_WHITEPOINT_XY; }
    [xyz[0] / sum, xyz[1] / sum]
}

/// Convert xy chromaticity to XYZ with Y = 1.
pub fn xy_to_xyz(xy: [f64; 2]) -> [f64; 3] {
    let x = xy[0];
    let y = xy[1];
    if !x.is_finite() || !y.is_finite() || y.abs() < 1e-30 {
        return xy_to_xyz(D65_WHITEPOINT_XY);
    }
    [x / y, 1.0, (1.0 - x - y) / y]
}

/// ProPhoto RGB → XYZ (D50) matrix.
pub fn prophoto_to_xyz(rgb: &[f64]) -> Vec<f64> {
    apply_matrix_to_triples(rgb, RgbColorSpace::ProPhotoRgb.spec().rgb_to_xyz)
}

/// Return the RGB → XYZ matrix for a colour space.
pub fn rgb_to_xyz_matrix(color_space: RgbColorSpace) -> [[f64; 3]; 3] {
    color_space.spec().rgb_to_xyz
}

/// Return the XYZ → RGB matrix for a colour space.
pub fn xyz_to_rgb_matrix(color_space: RgbColorSpace) -> [[f64; 3]; 3] {
    color_space.spec().xyz_to_rgb
}

/// Decode RGB values with the colour space transfer function.
///
/// ACES2065-1 is linear and therefore returned unchanged. Display P3 uses the
/// sRGB electro-optical transfer function, matching common Display P3 practice.
pub fn cctf_decode(encoded: &[f64], color_space: RgbColorSpace) -> Vec<f64> {
    encoded.iter().map(|&v| cctf_decode_value(v, color_space)).collect()
}

/// Encode RGB values with the colour space transfer function.
///
/// Values are not clamped, preserving negative/out-of-gamut values for linear
/// colour processing. Use `srgb_gamma_encode` when the previous clamped sRGB
/// behaviour is desired.
pub fn cctf_encode(linear: &[f64], color_space: RgbColorSpace) -> Vec<f64> {
    linear.iter().map(|&v| cctf_encode_value(v, color_space)).collect()
}

/// Decode a single component with the colour space transfer function.
pub fn cctf_decode_value(value: f64, color_space: RgbColorSpace) -> f64 {
    match color_space {
        RgbColorSpace::Srgb | RgbColorSpace::DisplayP3 => srgb_decode_value(value),
        RgbColorSpace::ProPhotoRgb => prophoto_decode_value(value),
        RgbColorSpace::Rec2020 => rec2020_decode_value(value),
        RgbColorSpace::Aces2065_1 => value,
    }
}

/// Encode a single component with the colour space transfer function.
pub fn cctf_encode_value(value: f64, color_space: RgbColorSpace) -> f64 {
    match color_space {
        RgbColorSpace::Srgb | RgbColorSpace::DisplayP3 => srgb_encode_value(value),
        RgbColorSpace::ProPhotoRgb => prophoto_encode_value(value),
        RgbColorSpace::Rec2020 => rec2020_encode_value(value),
        RgbColorSpace::Aces2065_1 => value,
    }
}

/// Convert RGB to XYZ in the RGB colour space reference white.
///
/// Set `apply_cctf_decoding` to true when input values are encoded image values,
/// mirroring `colour.RGB_to_XYZ(..., apply_cctf_decoding=True)`.
pub fn rgb_to_xyz(rgb: &[f64], color_space: RgbColorSpace, apply_cctf_decoding: bool) -> Vec<f64> {
    let linear = if apply_cctf_decoding {
        cctf_decode(rgb, color_space)
    } else {
        rgb.to_vec()
    };
    apply_matrix_to_triples(&linear, color_space.spec().rgb_to_xyz)
}

/// Convert RGB to XYZ and adapt from the RGB colour space white point to a
/// target illuminant white point.
pub fn rgb_to_xyz_with_adaptation(
    rgb: &[f64],
    color_space: RgbColorSpace,
    target_white_xy: [f64; 2],
    adaptation_transform: ChromaticAdaptationTransform,
    apply_cctf_decoding: bool,
) -> Vec<f64> {
    let xyz = rgb_to_xyz(rgb, color_space, apply_cctf_decoding);
    adapt_xyz(
        &xyz,
        color_space.spec().whitepoint_xy,
        target_white_xy,
        adaptation_transform,
    )
}

/// Convert XYZ in the RGB colour space reference white to RGB.
pub fn xyz_to_rgb(xyz: &[f64], color_space: RgbColorSpace, apply_cctf_encoding: bool) -> Vec<f64> {
    let linear = apply_matrix_to_triples(xyz, color_space.spec().xyz_to_rgb);
    if apply_cctf_encoding {
        cctf_encode(&linear, color_space)
    } else {
        linear
    }
}

/// Convert XYZ from a source illuminant white point to RGB, adapting to the RGB
/// colour space reference white before applying the colour space matrix.
pub fn xyz_to_rgb_with_adaptation(
    xyz: &[f64],
    source_white_xy: [f64; 2],
    color_space: RgbColorSpace,
    adaptation_transform: ChromaticAdaptationTransform,
    apply_cctf_encoding: bool,
) -> Vec<f64> {
    let adapted = adapt_xyz(
        xyz,
        source_white_xy,
        color_space.spec().whitepoint_xy,
        adaptation_transform,
    );
    xyz_to_rgb(&adapted, color_space, apply_cctf_encoding)
}

/// Convert RGB between two supported colour spaces with optional CCTF handling.
///
/// This mirrors the common `colour.RGB_to_RGB` flow: decode source RGB, convert
/// to XYZ, chromatically adapt between reference whites, convert to destination
/// RGB, and optionally encode the destination values.
pub fn rgb_to_rgb(
    rgb: &[f64],
    input_color_space: RgbColorSpace,
    output_color_space: RgbColorSpace,
    apply_cctf_decoding: bool,
    apply_cctf_encoding: bool,
    adaptation_transform: ChromaticAdaptationTransform,
) -> Vec<f64> {
    if input_color_space == output_color_space {
        let linear = if apply_cctf_decoding {
            cctf_decode(rgb, input_color_space)
        } else {
            rgb.to_vec()
        };
        return if apply_cctf_encoding {
            cctf_encode(&linear, output_color_space)
        } else {
            linear
        };
    }

    let xyz = rgb_to_xyz(rgb, input_color_space, apply_cctf_decoding);
    let adapted = adapt_xyz(
        &xyz,
        input_color_space.spec().whitepoint_xy,
        output_color_space.spec().whitepoint_xy,
        adaptation_transform,
    );
    xyz_to_rgb(&adapted, output_color_space, apply_cctf_encoding)
}

/// Convert RGB to ACES2065-1 linear AP0 RGB.
///
/// CAT02 is used by default to match common `colour-science` RGB conversion
/// behaviour when moving between differently white-balanced RGB colour spaces.
pub fn rgb_to_aces2065_1(
    rgb: &[f64],
    color_space: RgbColorSpace,
    apply_cctf_decoding: bool,
) -> Vec<f64> {
    rgb_to_rgb(
        rgb,
        color_space,
        RgbColorSpace::Aces2065_1,
        apply_cctf_decoding,
        false,
        ChromaticAdaptationTransform::Cat02,
    )
}

/// Convert XYZ values from one illuminant white point to another.
pub fn adapt_xyz(
    xyz: &[f64],
    source_white_xy: [f64; 2],
    target_white_xy: [f64; 2],
    adaptation_transform: ChromaticAdaptationTransform,
) -> Vec<f64> {
    if approximately_equal_xy(source_white_xy, target_white_xy) {
        return xyz.to_vec();
    }
    let matrix = chromatic_adaptation_matrix(source_white_xy, target_white_xy, adaptation_transform);
    apply_matrix_to_triples(xyz, matrix)
}

/// Build an XYZ chromatic adaptation matrix between two white points.
pub fn chromatic_adaptation_matrix(
    source_white_xy: [f64; 2],
    target_white_xy: [f64; 2],
    adaptation_transform: ChromaticAdaptationTransform,
) -> [[f64; 3]; 3] {
    if approximately_equal_xy(source_white_xy, target_white_xy) {
        return IDENTITY_3;
    }

    let (cat, cat_inv) = match adaptation_transform {
        ChromaticAdaptationTransform::Bradford => (
            [
                [0.8951, 0.2664, -0.1614],
                [-0.7502, 1.7135, 0.0367],
                [0.0389, -0.0685, 1.0296],
            ],
            [
                [0.9869929054667123, -0.1470542564209901, 0.1599626516637312],
                [0.4323052697233946, 0.5183602715367776, 0.0492912282128556],
                [-0.0085286645751773, 0.0400428216540849, 0.96848669578755],
            ],
        ),
        ChromaticAdaptationTransform::Cat02 => (
            [
                [0.7328, 0.4296, -0.1624],
                [-0.7036, 1.6975, 0.0061],
                [0.0030, 0.0136, 0.9834],
            ],
            [
                [1.096123820835514, -0.278869000218287, 0.182745179382773],
                [0.454369041975359, 0.473533154307412, 0.072097803717229],
                [-0.009627608738429, -0.005698031216113, 1.015325639954543],
            ],
        ),
    };

    let source_cone = mat3_vec_mul(cat, xy_to_xyz(source_white_xy));
    let target_cone = mat3_vec_mul(cat, xy_to_xyz(target_white_xy));
    let scale = [
        safe_div(target_cone[0], source_cone[0]),
        safe_div(target_cone[1], source_cone[1]),
        safe_div(target_cone[2], source_cone[2]),
    ];
    let diagonal = [
        [scale[0], 0.0, 0.0],
        [0.0, scale[1], 0.0],
        [0.0, 0.0, scale[2]],
    ];

    mat3_mul(mat3_mul(cat_inv, diagonal), cat)
}

/// XYZ D50 → D65 chromatic adaptation (Bradford transform).
pub fn xyz_d50_to_d65(xyz: &[f64]) -> Vec<f64> {
    const M: [[f64; 3]; 3] = [
        [0.9554734527042182, -0.023098536874261423, 0.0632593086610217],
        [-0.028369706963208136, 1.009995977820948, 0.021041398966943977],
        [0.012314001688319899, -0.020507696433477912, 1.3303659366080753],
    ];
    apply_matrix_to_triples(xyz, M)
}

/// XYZ D65 → D50 chromatic adaptation (Bradford transform).
pub fn xyz_d65_to_d50(xyz: &[f64]) -> Vec<f64> {
    const M: [[f64; 3]; 3] = [
        [1.0479298208405488, 0.022946793341019088, -0.05019222954313557],
        [0.029627815688159344, 0.990434484573249, -0.01707382502938514],
        [-0.009243058152591178, 0.015055144896577895, 0.7518742899580008],
    ];
    apply_matrix_to_triples(xyz, M)
}

/// Apply an ACES2065-1 to raw conversion matrix to flat ACES RGB triples.
///
/// Matrix layout follows the Python helper's `contract('ijk,lk->ijl', aces, M)`:
/// `matrix[raw_channel][aces_channel]`.
pub fn aces2065_1_to_raw_with_matrix(
    aces: &[f64],
    aces_to_raw_conversion_matrix: &[[f64; 3]; 3],
    midgray_rgb: [f64; 3],
) -> Vec<f64> {
    let n_px = aces.len() / 3;
    let mut out = vec![0.0f64; n_px * 3];
    for px in 0..n_px {
        for raw_ch in 0..3 {
            let mut v = 0.0;
            for aces_ch in 0..3 {
                v += aces[px * 3 + aces_ch] * aces_to_raw_conversion_matrix[raw_ch][aces_ch];
            }
            let midgray = midgray_rgb[raw_ch];
            out[px * 3 + raw_ch] = if midgray.abs() > 1e-30 { v / midgray } else { v };
        }
    }
    out
}

/// Convert RGB values to raw values using an externally computed ACES IDT matrix.
///
/// This is the Rust equivalent of the Python path that calls
/// `colour.RGB_to_RGB(..., 'ACES2065-1')` and then applies an
/// `aces_to_raw_conversion_matrix`. Exact `colour.matrix_idt` fitting requires
/// colour-science datasets and optimisation code, so the matrix is supplied by
/// the caller or estimated with [`compute_aces_to_raw_conversion_matrix_approx`].
pub fn rgb_to_raw_aces_idt_with_matrix(
    rgb: &[f64],
    aces_to_raw_conversion_matrix: &[[f64; 3]; 3],
    midgray_rgb: [f64; 3],
    color_space: RgbColorSpace,
    apply_cctf_decoding: bool,
) -> (Vec<f64>, [f64; 3]) {
    let aces = rgb_to_aces2065_1(rgb, color_space, apply_cctf_decoding);
    let raw = aces2065_1_to_raw_with_matrix(&aces, aces_to_raw_conversion_matrix, midgray_rgb);
    (raw, [1.0, 1.0, 1.0])
}

/// Estimate an ACES2065-1 → raw matrix from sensor sensitivities, illuminant and CMFs.
///
/// This is a practical least-squares approximation for cases where the exact
/// `colour.matrix_idt` procedure is unavailable. It fits a linear XYZ → raw
/// operator from per-wavelength samples and composes it with the ACES2065-1
/// RGB → XYZ matrix. Matrix layout matches [`aces2065_1_to_raw_with_matrix`].
pub fn compute_aces_to_raw_conversion_matrix_approx(
    sensitivity: &[[f64; 3]; N_WL],
    illuminant: &[f64; N_WL],
    cmfs: &[[f64; 3]; N_WL],
    normalization: f64,
    n_wl: usize,
) -> [[f64; 3]; 3] {
    let norm = if normalization > 1e-30 {
        normalization
    } else {
        let mut y = 0.0;
        for wl in 0..n_wl {
            y += illuminant[wl] * cmfs[wl][1];
        }
        if y > 1e-30 { y } else { 1.0 }
    };

    let mut xtx = [[0.0f64; 3]; 3];
    let mut xtr = [[0.0f64; 3]; 3];
    for wl in 0..n_wl {
        let x = [
            illuminant[wl] * cmfs[wl][0] / norm,
            illuminant[wl] * cmfs[wl][1] / norm,
            illuminant[wl] * cmfs[wl][2] / norm,
        ];
        let raw = [
            illuminant[wl] * sensitivity[wl][0],
            illuminant[wl] * sensitivity[wl][1],
            illuminant[wl] * sensitivity[wl][2],
        ];

        for i in 0..3 {
            for j in 0..3 {
                xtx[i][j] += x[i] * x[j];
            }
            for raw_ch in 0..3 {
                xtr[i][raw_ch] += x[i] * raw[raw_ch];
            }
        }
    }

    let xtx_inv = mat3_inverse(xtx).unwrap_or(IDENTITY_3);
    let xyz_to_raw_columns = mat3_mul(xtx_inv, xtr);
    let aces_to_xyz = RgbColorSpace::Aces2065_1.spec().rgb_to_xyz;
    let mut out = [[0.0f64; 3]; 3];

    for raw_ch in 0..3 {
        for aces_ch in 0..3 {
            let mut v = 0.0;
            for xyz_ch in 0..3 {
                v += xyz_to_raw_columns[xyz_ch][raw_ch] * aces_to_xyz[xyz_ch][aces_ch];
            }
            out[raw_ch][aces_ch] = v;
        }
    }

    out
}

fn apply_matrix_to_triples(values: &[f64], matrix: [[f64; 3]; 3]) -> Vec<f64> {
    let n_px = values.len() / 3;
    let mut out = vec![0.0f64; n_px * 3];
    for px in 0..n_px {
        for row in 0..3 {
            let mut v = 0.0;
            for col in 0..3 { v += matrix[row][col] * values[px * 3 + col]; }
            out[px * 3 + row] = v;
        }
    }
    out
}

fn srgb_encode_value(value: f64) -> f64 {
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let v = value.abs();
    let encoded = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    sign * encoded
}

fn srgb_decode_value(value: f64) -> f64 {
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let v = value.abs();
    let decoded = if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    };
    sign * decoded
}

fn prophoto_encode_value(value: f64) -> f64 {
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let v = value.abs();
    let encoded = if v < 1.0 / 512.0 {
        v * 16.0
    } else {
        v.powf(1.0 / 1.8)
    };
    sign * encoded
}

fn prophoto_decode_value(value: f64) -> f64 {
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let v = value.abs();
    let decoded = if v < 1.0 / 32.0 {
        v / 16.0
    } else {
        v.powf(1.8)
    };
    sign * decoded
}

fn rec2020_encode_value(value: f64) -> f64 {
    const ALPHA: f64 = 1.099_296_826_809_44;
    const BETA: f64 = 0.018_053_968_510_807;
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let v = value.abs();
    let encoded = if v < BETA {
        4.5 * v
    } else {
        ALPHA * v.powf(0.45) - (ALPHA - 1.0)
    };
    sign * encoded
}

fn rec2020_decode_value(value: f64) -> f64 {
    const ALPHA: f64 = 1.099_296_826_809_44;
    const BETA: f64 = 0.018_053_968_510_807;
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let v = value.abs();
    let decoded = if v < 4.5 * BETA {
        v / 4.5
    } else {
        ((v + (ALPHA - 1.0)) / ALPHA).powf(1.0 / 0.45)
    };
    sign * decoded
}

fn approximately_equal_xy(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12
}

fn safe_div(numerator: f64, denominator: f64) -> f64 {
    if denominator.abs() > 1e-30 { numerator / denominator } else { 1.0 }
}

fn mat3_vec_mul(matrix: [[f64; 3]; 3], value: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * value[0] + matrix[0][1] * value[1] + matrix[0][2] * value[2],
        matrix[1][0] * value[0] + matrix[1][1] * value[1] + matrix[1][2] * value[2],
        matrix[2][0] * value[0] + matrix[2][1] * value[1] + matrix[2][2] * value[2],
    ]
}

fn mat3_mul(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut out = [[0.0f64; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            for k in 0..3 {
                out[row][col] += a[row][k] * b[k][col];
            }
        }
    }
    out
}

fn mat3_inverse(m: [[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);

    if det.abs() < 1e-30 || !det.is_finite() {
        return None;
    }

    let inv_det = 1.0 / det;
    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
        ],
    ])
}
