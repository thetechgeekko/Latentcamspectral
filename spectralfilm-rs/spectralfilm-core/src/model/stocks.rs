//! Known film stock, print-paper, and profile string constants.
//!
//! Mirrors `spektrafilm/model/stocks.py` and the profile enum constants in
//! `spektrafilm/profiles/io.py`.

use core::str::FromStr;

// ---------------------------------------------------------------------------
// Profile field constants (`profiles/io.py`)
// ---------------------------------------------------------------------------

pub const PROFILE_TYPE_NEGATIVE: &str = "negative";
pub const PROFILE_TYPE_POSITIVE: &str = "positive";
pub const PROFILE_TYPES: [&str; 2] = [PROFILE_TYPE_NEGATIVE, PROFILE_TYPE_POSITIVE];

pub const PROFILE_SUPPORT_FILM: &str = "film";
pub const PROFILE_SUPPORT_PAPER: &str = "paper";
pub const PROFILE_SUPPORTS: [&str; 2] = [PROFILE_SUPPORT_FILM, PROFILE_SUPPORT_PAPER];

pub const PROFILE_STAGE_FILMING: &str = "filming";
pub const PROFILE_STAGE_PRINTING: &str = "printing";
pub const PROFILE_STAGES: [&str; 2] = [PROFILE_STAGE_FILMING, PROFILE_STAGE_PRINTING];

pub const PROFILE_USE_STILL: &str = "still";
pub const PROFILE_USE_CINE: &str = "cine";
pub const PROFILE_USES: [&str; 2] = [PROFILE_USE_STILL, PROFILE_USE_CINE];

pub const PROFILE_ANTIHALATION_STRONG: &str = "strong";
pub const PROFILE_ANTIHALATION_WEAK: &str = "weak";
pub const PROFILE_ANTIHALATION_NO: &str = "no";
pub const PROFILE_ANTIHALATION: [&str; 3] = [
    PROFILE_ANTIHALATION_STRONG,
    PROFILE_ANTIHALATION_WEAK,
    PROFILE_ANTIHALATION_NO,
];

pub const PROFILE_CHANNEL_MODEL_COLOR: &str = "color";
pub const PROFILE_CHANNEL_MODEL_BW: &str = "bw";
pub const PROFILE_CHANNEL_MODELS: [&str; 2] = [PROFILE_CHANNEL_MODEL_COLOR, PROFILE_CHANNEL_MODEL_BW];

// ---------------------------------------------------------------------------
// Film stock constants (`model/stocks.py`)
// ---------------------------------------------------------------------------

pub const KODAK_EKTAR_100: &str = "kodak_ektar_100";
pub const KODAK_PORTRA_160: &str = "kodak_portra_160";
pub const KODAK_PORTRA_400: &str = "kodak_portra_400";
pub const KODAK_PORTRA_800: &str = "kodak_portra_800";
pub const KODAK_PORTRA_800_PUSH1: &str = "kodak_portra_800_push1";
pub const KODAK_PORTRA_800_PUSH2: &str = "kodak_portra_800_push2";
pub const KODAK_GOLD_200: &str = "kodak_gold_200";
pub const KODAK_ULTRAMAX_400: &str = "kodak_ultramax_400";
pub const KODAK_VISION3_50D: &str = "kodak_vision3_50d";
pub const KODAK_VISION3_250D: &str = "kodak_vision3_250d";
pub const KODAK_VERITA_200D: &str = "kodak_verita_200d";
pub const KODAK_VISION3_200T: &str = "kodak_vision3_200t";
pub const KODAK_VISION3_500T: &str = "kodak_vision3_500t";
pub const FUJIFILM_PRO_400H: &str = "fujifilm_pro_400h";
pub const FUJIFILM_C200: &str = "fujifilm_c200";
pub const FUJIFILM_XTRA_400: &str = "fujifilm_xtra_400";
pub const KODAK_EKTACHROME_100: &str = "kodak_ektachrome_100";
pub const KODAK_KODACHROME_64: &str = "kodak_kodachrome_64";
pub const FUJIFILM_VELVIA_100: &str = "fujifilm_velvia_100";
pub const FUJIFILM_PROVIA_100F: &str = "fujifilm_provia_100f";

pub const FILM_STOCK_NAMES: [&str; 20] = [
    KODAK_EKTAR_100,
    KODAK_PORTRA_160,
    KODAK_PORTRA_400,
    KODAK_PORTRA_800,
    KODAK_PORTRA_800_PUSH1,
    KODAK_PORTRA_800_PUSH2,
    KODAK_GOLD_200,
    KODAK_ULTRAMAX_400,
    KODAK_VISION3_50D,
    KODAK_VISION3_250D,
    KODAK_VERITA_200D,
    KODAK_VISION3_200T,
    KODAK_VISION3_500T,
    FUJIFILM_PRO_400H,
    FUJIFILM_C200,
    FUJIFILM_XTRA_400,
    KODAK_EKTACHROME_100,
    KODAK_KODACHROME_64,
    FUJIFILM_VELVIA_100,
    FUJIFILM_PROVIA_100F,
];

/// Known film stocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilmStock {
    KodakEktar100,
    KodakPortra160,
    KodakPortra400,
    KodakPortra800,
    KodakPortra800Push1,
    KodakPortra800Push2,
    KodakGold200,
    KodakUltramax400,
    KodakVision350D,
    KodakVision3250D,
    KodakVerita200D,
    KodakVision3200T,
    KodakVision3500T,
    FujifilmPro400H,
    FujifilmC200,
    FujifilmXtra400,
    KodakEktachrome100,
    KodakKodachrome64,
    FujifilmVelvia100,
    FujifilmProvia100F,
}

impl FilmStock {
    pub const ALL: [Self; 20] = [
        Self::KodakEktar100,
        Self::KodakPortra160,
        Self::KodakPortra400,
        Self::KodakPortra800,
        Self::KodakPortra800Push1,
        Self::KodakPortra800Push2,
        Self::KodakGold200,
        Self::KodakUltramax400,
        Self::KodakVision350D,
        Self::KodakVision3250D,
        Self::KodakVerita200D,
        Self::KodakVision3200T,
        Self::KodakVision3500T,
        Self::FujifilmPro400H,
        Self::FujifilmC200,
        Self::FujifilmXtra400,
        Self::KodakEktachrome100,
        Self::KodakKodachrome64,
        Self::FujifilmVelvia100,
        Self::FujifilmProvia100F,
    ];

    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KodakEktar100 => KODAK_EKTAR_100,
            Self::KodakPortra160 => KODAK_PORTRA_160,
            Self::KodakPortra400 => KODAK_PORTRA_400,
            Self::KodakPortra800 => KODAK_PORTRA_800,
            Self::KodakPortra800Push1 => KODAK_PORTRA_800_PUSH1,
            Self::KodakPortra800Push2 => KODAK_PORTRA_800_PUSH2,
            Self::KodakGold200 => KODAK_GOLD_200,
            Self::KodakUltramax400 => KODAK_ULTRAMAX_400,
            Self::KodakVision350D => KODAK_VISION3_50D,
            Self::KodakVision3250D => KODAK_VISION3_250D,
            Self::KodakVerita200D => KODAK_VERITA_200D,
            Self::KodakVision3200T => KODAK_VISION3_200T,
            Self::KodakVision3500T => KODAK_VISION3_500T,
            Self::FujifilmPro400H => FUJIFILM_PRO_400H,
            Self::FujifilmC200 => FUJIFILM_C200,
            Self::FujifilmXtra400 => FUJIFILM_XTRA_400,
            Self::KodakEktachrome100 => KODAK_EKTACHROME_100,
            Self::KodakKodachrome64 => KODAK_KODACHROME_64,
            Self::FujifilmVelvia100 => FUJIFILM_VELVIA_100,
            Self::FujifilmProvia100F => FUJIFILM_PROVIA_100F,
        }
    }

    #[inline]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            KODAK_EKTAR_100 => Some(Self::KodakEktar100),
            KODAK_PORTRA_160 => Some(Self::KodakPortra160),
            KODAK_PORTRA_400 => Some(Self::KodakPortra400),
            KODAK_PORTRA_800 => Some(Self::KodakPortra800),
            KODAK_PORTRA_800_PUSH1 => Some(Self::KodakPortra800Push1),
            KODAK_PORTRA_800_PUSH2 => Some(Self::KodakPortra800Push2),
            KODAK_GOLD_200 => Some(Self::KodakGold200),
            KODAK_ULTRAMAX_400 => Some(Self::KodakUltramax400),
            KODAK_VISION3_50D => Some(Self::KodakVision350D),
            KODAK_VISION3_250D => Some(Self::KodakVision3250D),
            KODAK_VERITA_200D => Some(Self::KodakVerita200D),
            KODAK_VISION3_200T => Some(Self::KodakVision3200T),
            KODAK_VISION3_500T => Some(Self::KodakVision3500T),
            FUJIFILM_PRO_400H => Some(Self::FujifilmPro400H),
            FUJIFILM_C200 => Some(Self::FujifilmC200),
            FUJIFILM_XTRA_400 => Some(Self::FujifilmXtra400),
            KODAK_EKTACHROME_100 => Some(Self::KodakEktachrome100),
            KODAK_KODACHROME_64 => Some(Self::KodakKodachrome64),
            FUJIFILM_VELVIA_100 => Some(Self::FujifilmVelvia100),
            FUJIFILM_PROVIA_100F => Some(Self::FujifilmProvia100F),
            _ => None,
        }
    }
}

impl From<FilmStock> for &'static str {
    #[inline]
    fn from(value: FilmStock) -> Self {
        value.as_str()
    }
}

impl FromStr for FilmStock {
    type Err = UnknownStockName;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s).ok_or(UnknownStockName)
    }
}

// ---------------------------------------------------------------------------
// Print paper constants (`model/stocks.py`)
// ---------------------------------------------------------------------------

pub const KODAK_ULTRA_ENDURA: &str = "kodak_ultra_endura";
pub const KODAK_ENDURA_PREMIER: &str = "kodak_endura_premier";
pub const KODAK_EKTACOLOR_EDGE: &str = "kodak_ektacolor_edge";
pub const KODAK_SUPRA_ENDURA: &str = "kodak_supra_endura";
pub const KODAK_PORTRA_ENDURA: &str = "kodak_portra_endura";
pub const FUJIFILM_CRYSTAL_ARCHIVE_TYPEII: &str = "fujifilm_crystal_archive_typeii";
pub const KODAK_2383: &str = "kodak_2383";
pub const KODAK_2393: &str = "kodak_2393";

pub const PRINT_PAPER_NAMES: [&str; 8] = [
    KODAK_ULTRA_ENDURA,
    KODAK_ENDURA_PREMIER,
    KODAK_EKTACOLOR_EDGE,
    KODAK_SUPRA_ENDURA,
    KODAK_PORTRA_ENDURA,
    FUJIFILM_CRYSTAL_ARCHIVE_TYPEII,
    KODAK_2383,
    KODAK_2393,
];

/// Known print papers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrintPaper {
    KodakUltraEndura,
    KodakEnduraPremier,
    KodakEktacolorEdge,
    KodakSupraEndura,
    KodakPortraEndura,
    FujifilmCrystalArchiveTypeIi,
    Kodak2383,
    Kodak2393,
}

impl PrintPaper {
    pub const ALL: [Self; 8] = [
        Self::KodakUltraEndura,
        Self::KodakEnduraPremier,
        Self::KodakEktacolorEdge,
        Self::KodakSupraEndura,
        Self::KodakPortraEndura,
        Self::FujifilmCrystalArchiveTypeIi,
        Self::Kodak2383,
        Self::Kodak2393,
    ];

    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KodakUltraEndura => KODAK_ULTRA_ENDURA,
            Self::KodakEnduraPremier => KODAK_ENDURA_PREMIER,
            Self::KodakEktacolorEdge => KODAK_EKTACOLOR_EDGE,
            Self::KodakSupraEndura => KODAK_SUPRA_ENDURA,
            Self::KodakPortraEndura => KODAK_PORTRA_ENDURA,
            Self::FujifilmCrystalArchiveTypeIi => FUJIFILM_CRYSTAL_ARCHIVE_TYPEII,
            Self::Kodak2383 => KODAK_2383,
            Self::Kodak2393 => KODAK_2393,
        }
    }

    #[inline]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            KODAK_ULTRA_ENDURA => Some(Self::KodakUltraEndura),
            KODAK_ENDURA_PREMIER => Some(Self::KodakEnduraPremier),
            KODAK_EKTACOLOR_EDGE => Some(Self::KodakEktacolorEdge),
            KODAK_SUPRA_ENDURA => Some(Self::KodakSupraEndura),
            KODAK_PORTRA_ENDURA => Some(Self::KodakPortraEndura),
            FUJIFILM_CRYSTAL_ARCHIVE_TYPEII => Some(Self::FujifilmCrystalArchiveTypeIi),
            KODAK_2383 => Some(Self::Kodak2383),
            KODAK_2393 => Some(Self::Kodak2393),
            _ => None,
        }
    }
}

impl From<PrintPaper> for &'static str {
    #[inline]
    fn from(value: PrintPaper) -> Self {
        value.as_str()
    }
}

impl FromStr for PrintPaper {
    type Err = UnknownStockName;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s).ok_or(UnknownStockName)
    }
}

/// Error returned when parsing an unknown film stock or print paper name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnknownStockName;

impl core::fmt::Display for UnknownStockName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("unknown stock or print paper name")
    }
}

impl std::error::Error for UnknownStockName {}

/// Return all known film stock profile names.
#[inline]
pub const fn all_film_stock_names() -> &'static [&'static str; 20] {
    &FILM_STOCK_NAMES
}

/// Return all known print paper profile names.
#[inline]
pub const fn all_print_paper_names() -> &'static [&'static str; 8] {
    &PRINT_PAPER_NAMES
}

/// Return all known profile names for film stocks and print papers.
pub fn all_known_stock_names() -> Vec<&'static str> {
    FILM_STOCK_NAMES
        .iter()
        .chain(PRINT_PAPER_NAMES.iter())
        .copied()
        .collect()
}
