use async_trait::async_trait;
use log::debug;

use crate::{pipeline::{Pipeline, feeder::PipelineFeeder}, traits::FrameProcessor, types::FrameData};

pub struct OnErrorSwitch {
    feeder: PipelineFeeder
}

impl OnErrorSwitch {
    pub fn new(destination_pipeline: &mut Pipeline) -> Self {
        Self {
            feeder: destination_pipeline.get_feeder()
        }
    }
}

#[async_trait]
impl FrameProcessor for OnErrorSwitch {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        debug!("Drop reason: {:?}", frame_data.get_drop_reason());

        if frame_data.get_drop_reason().is_some() {
            debug!("Feeding frame");
            self.feeder.feed(frame_data);
            None
        } else {
            Some(frame_data)
        }
    }
}
