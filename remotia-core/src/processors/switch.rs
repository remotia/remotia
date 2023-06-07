use async_trait::async_trait;
use log::debug;

use crate::{pipeline::{Pipeline, feeder::PipelineFeeder}, traits::FrameProcessor, types::FrameData};

pub struct Switch {
    feeder: PipelineFeeder
}

impl Switch {
    pub fn new(destination_pipeline: &mut Pipeline) -> Self {
        Self {
            feeder: destination_pipeline.get_feeder()
        }
    }
}

#[async_trait]
impl FrameProcessor for Switch {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        self.feeder.feed(frame_data);
        None
    }
}
