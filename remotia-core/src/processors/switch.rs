use std::fmt::Debug;

use async_trait::async_trait;
use log::debug;

use crate::{
    pipeline::{feeder::PipelineFeeder, Pipeline},
    traits::FrameProcessor,
};

pub struct Switch<F> {
    feeder: PipelineFeeder<F>,
}

impl<F> Switch<F>
where
    F: Default + Debug + Send + 'static,
{
    pub fn new(destination_pipeline: &mut Pipeline<F>) -> Self {
        Self {
            feeder: destination_pipeline.get_feeder(),
        }
    }
}

#[async_trait]
impl<F> FrameProcessor<F> for Switch<F>
where
    F: Debug + Send,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        self.feeder.feed(frame_data);
        None
    }
}
