use async_trait::async_trait;

use remotia_core::{
    common::helpers::time::now_timestamp,
    traits::{FrameProcessor, FrameProperties},
};

pub struct TimestampAdder<K> {
    id: K,
}

impl<K> TimestampAdder<K> {
    pub fn new(id: K) -> Self {
        Self { id }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for TimestampAdder<K>
where
    K: Copy + Send,
    F: FrameProperties<K, u128> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        frame_data.set(self.id, now_timestamp());
        Some(frame_data)
    }
}
