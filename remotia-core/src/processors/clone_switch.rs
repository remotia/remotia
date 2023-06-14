use std::fmt::Debug;
use async_trait::async_trait;

use crate::{pipeline::{Pipeline, feeder::PipelineFeeder}, traits::FrameProcessor};

pub struct CloneSwitch<F> {
    feeder: PipelineFeeder<F>
}

impl<F> CloneSwitch<F> {
    pub fn new(destination_pipeline: &mut Pipeline<F>) -> Self where
        F: Debug + Default + Send + 'static
    {
        Self {
            feeder: destination_pipeline.get_feeder()
        }
    }
}

#[async_trait]
impl<F> FrameProcessor<F> for CloneSwitch<F> where
    F: Debug + Clone + Default + Send + 'static
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        self.feeder.feed(frame_data.clone());
        Some(frame_data)
    }
}
