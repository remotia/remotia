use std::{collections::HashMap, fmt::Debug, hash::Hash};

use async_trait::async_trait;
use log::debug;
use rand::prelude::{SliceRandom, ThreadRng};

use crate::{
    pipeline::{feeder::PipelineFeeder, Pipeline},
    traits::{FrameProcessor, FrameProperties},
};

pub struct PoolingSwitch<F, P, K> {
    property_key: P,
    entries: Vec<(K, PipelineFeeder<F>)>
}

impl<F, P, K> PoolingSwitch<F, P, K> where
    F: Debug + Default + Send
{
    pub fn new(property_key: P) -> Self {
        Self {
            property_key,
            entries: Vec::new(),
        }
    }

    pub fn entry(mut self, key: K, pipeline: &mut Pipeline<F>) -> Self where
        F: 'static
    {
        self.entries.push((key, pipeline.get_feeder()));
        self
    }
}

#[async_trait]
impl<F, P, K> FrameProcessor<F> for PoolingSwitch<F, P, K> where
    P: Copy + Send,
    K: Copy + Send,
    F: Debug + FrameProperties<P, K> + Send + 'static
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let (key, feeder) = self.entries.choose(&mut rand::thread_rng()).unwrap();

        frame_data.set(self.property_key, *key);
        feeder.feed(frame_data);

        None
    }
}

pub struct DepoolingSwitch<F, P, K> {
    property_key: P,
    entries: HashMap<K, PipelineFeeder<F>>
}

impl<F, P, K> DepoolingSwitch<F, P, K> {
    pub fn new(property_key: P) -> Self {
        Self {
            property_key,
            entries: HashMap::new(),
        }
    }

    pub fn entry(mut self, key: K, pipeline: &mut Pipeline<F>) -> Self where
        K: Hash + Eq,
        F: Debug + Default + Send + 'static
    {
        self.entries.insert(key, pipeline.get_feeder());
        self
    }
}

#[async_trait]
impl<F, P, K> FrameProcessor<F> for DepoolingSwitch<F, P, K> 
where
    P: Send,
    K: Eq + Hash + Send,
    F: Debug + FrameProperties<P, K> + Send + 'static
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let key = frame_data.get(&self.property_key).unwrap();
        let feeder = self.entries.get(&key).unwrap();

        feeder.feed(frame_data);

        None
    }
}

