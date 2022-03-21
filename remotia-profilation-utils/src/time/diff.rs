use async_trait::async_trait;

use remotia_core::{common::helpers::time::now_timestamp, traits::FrameProcessor, types::FrameData};

pub struct TimestampDiffCalculator {
    source_id: String,
    diff_id: String,
}

impl TimestampDiffCalculator {
    pub fn new(source_id: &str, diff_id: &str) -> Self {
        Self { 
            source_id: source_id.to_string(), 
            diff_id: diff_id.to_string()
        }
    }
}

#[async_trait]
impl FrameProcessor for TimestampDiffCalculator {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let source_timestamp = frame_data.get(&self.source_id);
        frame_data.set(&self.diff_id, now_timestamp() - source_timestamp);
        Some(frame_data)
    }
}
