//! Lightweight timing helpers.

use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Default, Clone)]
pub struct Timings {
    values: HashMap<String, f64>,
}

impl Timings {
    pub fn new() -> Self { Self::default() }
    pub fn clear(&mut self) { self.values.clear(); }
    pub fn record(&mut self, key: impl Into<String>, seconds: f64) { self.values.insert(key.into(), seconds); }
    pub fn values(&self) -> &HashMap<String, f64> { &self.values }
    pub fn format(&self, total_elapsed_time: Option<f64>) -> String {
        let mut rows: Vec<_> = self.values.iter().collect();
        rows.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut out = String::from("Simulation timings\n");
        for (name, seconds) in rows {
            out.push_str(&format!("{name:36} {:9.3} ms\n", seconds * 1000.0));
        }
        if let Some(total) = total_elapsed_time {
            out.push_str(&format!("{:<36} {:9.3} ms\n", "total", total * 1000.0));
        }
        out
    }
}

pub fn timed<T>(timings: &mut Timings, key: impl Into<String>, f: impl FnOnce() -> T) -> T {
    let key = key.into();
    let start = Instant::now();
    let value = f();
    timings.record(key, start.elapsed().as_secs_f64());
    value
}
