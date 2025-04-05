use async_trait::async_trait;

use remotia_core::{
    common::helpers::time::now_timestamp,
    traits::{FrameProcessor, FrameProperties},
};

pub struct TimestampDiffCalculator<K> {
    source_id: K,
    diff_id: K,
}

impl<K> TimestampDiffCalculator<K> {
    pub fn new(source_id: K, diff_id: K) -> Self {
        Self {
            source_id,
            diff_id,
        }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for TimestampDiffCalculator<K>
where
    K: Copy + Send,
    F: FrameProperties<K, u128> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let source_timestamp = frame_data.get(&self.source_id).unwrap();
        frame_data.set(self.diff_id, now_timestamp() - source_timestamp);
        Some(frame_data)
    }
}
