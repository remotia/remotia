use std::collections::HashMap;

use async_trait::async_trait;
use log::debug;
use rand::prelude::{SliceRandom, ThreadRng};

use crate::{
    pipeline::{feeder::PipelineFeeder, Pipeline},
    traits::FrameProcessor,
    types::FrameData,
};

pub struct PoolingSwitch {
    entries: Vec<(u128, PipelineFeeder)>
}

impl PoolingSwitch {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn entry(mut self, key: u128, pipeline: &mut Pipeline) -> Self {
        self.entries.push((key, pipeline.get_feeder()));
        self
    }
}

impl Default for PoolingSwitch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FrameProcessor for PoolingSwitch {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let (key, feeder) = self.entries.choose(&mut rand::thread_rng()).unwrap();

        debug!("Feeding to pipeline #{}...", key);

        frame_data.set("pool_key", *key);
        feeder.feed(frame_data);

        None
    }
}

pub struct DepoolingSwitch {
    entries: HashMap<u128, PipelineFeeder>
}

impl DepoolingSwitch {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn entry(mut self, key: u128, pipeline: &mut Pipeline) -> Self {
        self.entries.insert(key, pipeline.get_feeder());
        self
    }
}

impl Default for DepoolingSwitch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FrameProcessor for DepoolingSwitch {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        let key = frame_data.get("pool_key");
        let feeder = self.entries.get(&key).unwrap();

        debug!("Feeding to pipeline #{}...", key);

        feeder.feed(frame_data);

        None
    }
}

