//! User-facing process convenience helpers.

use crate::runtime::params::{RuntimeParamsPatch, RuntimePhotoParams};
use crate::runtime::params_builder::digest_params;
use crate::runtime::pipeline::{SimulationError, SimulationOutput, SimulationPipeline};

pub struct Simulator {
    params: RuntimePhotoParams,
    pipeline: SimulationPipeline,
}

impl Simulator {
    pub fn new(params: RuntimePhotoParams) -> Self {
        Self { pipeline: SimulationPipeline::new(params.clone()), params }
    }

    pub fn from_json(json: &str, digest: bool) -> Result<Self, serde_json::Error> {
        let params = RuntimePhotoParams::from_json(json)?;
        let params = if digest { digest_params(params) } else { params };
        Ok(Self::new(params))
    }

    pub fn process(&mut self, image: &[f64], width: usize, height: usize) -> Result<SimulationOutput, SimulationError> {
        self.pipeline.process(image, width, height)
    }

    pub fn update_params(&mut self, params: RuntimePhotoParams) {
        self.pipeline.update(params.clone());
        self.params = params;
    }

    pub fn update_params_json(&mut self, json: &str, digest: bool) -> Result<(), serde_json::Error> {
        let params = RuntimePhotoParams::from_json(json)?;
        let params = if digest { digest_params(params) } else { params };
        self.update_params(params);
        Ok(())
    }

    pub fn soft_update(&mut self, patch: RuntimeParamsPatch, digest: bool) {
        let mut params = self.params.clone();
        params.apply_patch(patch);
        let params = if digest { digest_params(params) } else { params };
        self.update_params(params);
    }

    pub fn soft_update_json(&mut self, json: &str, digest: bool) -> Result<(), serde_json::Error> {
        let patch = RuntimeParamsPatch::from_json(json)?;
        self.soft_update(patch, digest);
        Ok(())
    }

    pub fn params(&self) -> &RuntimePhotoParams { &self.params }

    pub fn params_json(&self) -> Result<String, serde_json::Error> { self.params.to_json() }

    pub fn format_timings(&self) -> String { self.pipeline.format_timings() }
}

pub fn simulate(image: &[f64], width: usize, height: usize, params: RuntimePhotoParams, digest: bool) -> Result<SimulationOutput, SimulationError> {
    let params = if digest { digest_params(params) } else { params };
    let mut simulator = Simulator::new(params);
    simulator.process(image, width, height)
}
