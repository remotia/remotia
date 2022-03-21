use async_trait::async_trait;

use remotia_core::{common::helpers::time::now_timestamp, traits::FrameProcessor, types::FrameData};

pub struct TimestampAdder {
    id: String,
}

impl TimestampAdder {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

#[async_trait]
impl FrameProcessor for TimestampAdder {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        frame_data.set(&self.id, now_timestamp());
        Some(frame_data)
    }
}
