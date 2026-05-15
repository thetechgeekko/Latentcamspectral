//! Spectral LUT cache service.
//!
//! This mirrors the Python `SpectralLUTService` helper: callers provide the
//! direct spectral calculation, and this service either runs it directly or
//! reuses/builds a cached 3-D LUT. Cache invalidation is intentionally based on
//! output equivalence for a small fixed CMY sample, matching the Python runtime
//! behaviour and avoiding a brittle dependency on every upstream parameter.

use crate::utils::lut::Lut3d;
use crate::utils::spectral_upsampling::RawLut2d;

const DEFAULT_CMY_TEST_VALUES: [f64; 12] = [
    0.1, 0.2, 0.3,
    0.4, 0.5, 0.6,
    0.7, 0.8, 0.9,
    1.0, 1.1, 1.2,
];

/// Caches spectral acceleration LUTs for filming, enlarging, and scanning.
///
/// `Lut3d` accelerates functions whose input is flat `[N × 3]` CMY data and
/// whose output is flat `[N × n_out]` data. The current runtime spectral
/// calculations produce three output channels, but the service keeps `n_out`
/// generic so the public API remains useful for future spectral paths.
#[derive(Debug, Clone)]
pub struct SpectralLUTService {
    lut_resolution: usize,

    /// Cached RGB/TC spectral upsampling LUT for the filming path, when a
    /// caller supplies a compatible LUT builder.
    pub filming_tc_lut_memory: Option<RawLut2d>,
    /// Cached CMY → print exposure/log-raw LUT for the enlarger path.
    pub enlarger_lut_memory: Option<Lut3d>,
    /// Cached CMY density → scanner output LUT for the scanner path.
    pub scanner_lut_memory: Option<Lut3d>,

    film_sensitivity: Option<Vec<f64>>,
    enlarger_test_results_memory: Option<Vec<f64>>,
    scanner_test_results_memory: Option<Vec<f64>>,
    cmy_test_values: Vec<f64>,
}

impl SpectralLUTService {
    /// Create a LUT cache service with the configured 3-D LUT resolution.
    pub fn new(lut_resolution: usize) -> Self {
        Self {
            lut_resolution,
            filming_tc_lut_memory: None,
            enlarger_lut_memory: None,
            scanner_lut_memory: None,
            film_sensitivity: None,
            enlarger_test_results_memory: None,
            scanner_test_results_memory: None,
            cmy_test_values: DEFAULT_CMY_TEST_VALUES.to_vec(),
        }
    }

    /// Return the configured LUT resolution.
    pub fn lut_resolution(&self) -> usize { self.lut_resolution }

    /// Update the LUT resolution and invalidate CMY-domain LUTs if it changed.
    pub fn set_lut_resolution(&mut self, lut_resolution: usize) {
        if self.lut_resolution == lut_resolution {
            return;
        }
        self.lut_resolution = lut_resolution;
        self.enlarger_lut_memory = None;
        self.scanner_lut_memory = None;
        self.enlarger_test_results_memory = None;
        self.scanner_test_results_memory = None;
    }

    /// Clear all cached LUTs and cache keys.
    pub fn clear(&mut self) {
        self.filming_tc_lut_memory = None;
        self.enlarger_lut_memory = None;
        self.scanner_lut_memory = None;
        self.film_sensitivity = None;
        self.enlarger_test_results_memory = None;
        self.scanner_test_results_memory = None;
    }

    /// Compute an enlarger spectral transform directly or through a cached LUT.
    ///
    /// `spectral_calculation` must accept flat `[N × 3]` CMY data and return a
    /// flat `[N × n_out]` output. When `use_lut` is false, this returns the
    /// direct spectral calculation without touching the cache.
    pub fn spectral_compute_enlarger<F>(
        &mut self,
        cmy_data: &[f64],
        spectral_calculation: F,
        data_min: [f64; 3],
        data_max: [f64; 3],
        use_lut: bool,
    ) -> Vec<f64>
    where
        F: Fn(&[f64]) -> Vec<f64> + Send + Sync,
    {
        if !use_lut {
            return spectral_calculation(cmy_data);
        }

        let cmy_test_values = self.cmy_test_values.clone();
        compute_with_cached_lut(
            cmy_data,
            &spectral_calculation,
            data_min,
            data_max,
            self.lut_resolution,
            &cmy_test_values,
            &mut self.enlarger_lut_memory,
            &mut self.enlarger_test_results_memory,
        )
    }

    /// Compute a scanner spectral transform directly or through a cached LUT.
    ///
    /// `spectral_calculation` must accept flat `[N × 3]` CMY data and return a
    /// flat `[N × n_out]` output. When `use_lut` is false, this returns the
    /// direct spectral calculation without touching the cache.
    pub fn spectral_compute_scanner<F>(
        &mut self,
        cmy_data: &[f64],
        spectral_calculation: F,
        data_min: [f64; 3],
        data_max: [f64; 3],
        use_lut: bool,
    ) -> Vec<f64>
    where
        F: Fn(&[f64]) -> Vec<f64> + Send + Sync,
    {
        if !use_lut {
            return spectral_calculation(cmy_data);
        }

        let cmy_test_values = self.cmy_test_values.clone();
        compute_with_cached_lut(
            cmy_data,
            &spectral_calculation,
            data_min,
            data_max,
            self.lut_resolution,
            &cmy_test_values,
            &mut self.scanner_lut_memory,
            &mut self.scanner_test_results_memory,
        )
    }

    /// Return a cached filming TC LUT, recomputing it when sensitivity changes.
    ///
    /// The Rust runtime currently exposes only the direct Hanatos-compatible
    /// spectral upsampling fallback, not the Python `compute_hanatos2025_tc_lut`
    /// builder. To keep the service usable without editing utilities, callers
    /// provide the builder closure and this method owns the cache/invalidation
    /// policy.
    pub fn get_filming_tc_lut<F>(&mut self, sensitivity: &[f64], compute_tc_lut: F) -> Option<&RawLut2d>
    where
        F: FnOnce(&[f64]) -> Option<RawLut2d>,
    {
        let cache_valid = self
                .film_sensitivity
                .as_ref()
                .is_some_and(|cached_sensitivity| exact_slice_eq(cached_sensitivity, sensitivity));

        if !cache_valid {
            self.film_sensitivity = Some(sensitivity.to_vec());
            self.filming_tc_lut_memory = compute_tc_lut(sensitivity);
        }

        self.filming_tc_lut_memory.as_ref()
    }
}

impl Default for SpectralLUTService {
    fn default() -> Self { Self::new(17) }
}

fn compute_with_cached_lut<F>(
    cmy_data: &[f64],
    spectral_calculation: &F,
    data_min: [f64; 3],
    data_max: [f64; 3],
    lut_resolution: usize,
    cmy_test_values: &[f64],
    lut_memory: &mut Option<Lut3d>,
    test_results_memory: &mut Option<Vec<f64>>,
) -> Vec<f64>
where
    F: Fn(&[f64]) -> Vec<f64> + Send + Sync,
{
    if !is_valid_lut_domain(data_min, data_max) || lut_resolution < 2 || cmy_data.len() % 3 != 0 {
        return spectral_calculation(cmy_data);
    }

    let test_results = spectral_calculation(cmy_test_values);
    let test_points = cmy_test_values.len() / 3;
    if test_points == 0 || test_results.len() % test_points != 0 {
        return spectral_calculation(cmy_data);
    }
    let n_out = test_results.len() / test_points;
    if n_out == 0 {
        return spectral_calculation(cmy_data);
    }

    let cache_valid = lut_memory
        .as_ref()
        .is_some_and(|lut| lut.steps == lut_resolution && lut.n_out == n_out)
        && test_results_memory
            .as_ref()
            .is_some_and(|cached_results| exact_slice_eq(cached_results, &test_results));

    if !cache_valid {
        *lut_memory = Some(Lut3d::build(lut_resolution, data_min, data_max, n_out, spectral_calculation));
        *test_results_memory = Some(test_results);
    }

    match lut_memory.as_ref() {
        Some(lut) => lut.apply(cmy_data),
        None => spectral_calculation(cmy_data),
    }
}

fn is_valid_lut_domain(data_min: [f64; 3], data_max: [f64; 3]) -> bool {
    (0..3).all(|ch| data_min[ch].is_finite() && data_max[ch].is_finite() && data_max[ch] > data_min[ch])
}

fn exact_slice_eq(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(av, bv)| av == bv)
}
