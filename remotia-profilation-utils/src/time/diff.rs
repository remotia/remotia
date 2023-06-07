use async_trait::async_trait;

use remotia_core::{
    common::helpers::time::now_timestamp,
    traits::{FrameProcessor, FrameProperties},
};

pub struct TimestampDiffCalculator {
    source_id: String,
    diff_id: String,
}

impl TimestampDiffCalculator {
    pub fn new(source_id: &str, diff_id: &str) -> Self {
        Self {
            source_id: source_id.to_string(),
            diff_id: diff_id.to_string(),
        }
    }
}

#[async_trait]
impl<F> FrameProcessor<F> for TimestampDiffCalculator
where
    F: FrameProperties<String, u128> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let source_timestamp = frame_data.get(&self.source_id).unwrap();
        frame_data.set(self.diff_id.clone(), now_timestamp() - source_timestamp);
        Some(frame_data)
    }
}
