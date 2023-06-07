use async_trait::async_trait;

use remotia_core::{
    common::helpers::time::now_timestamp,
    traits::{FrameProcessor, FrameProperties},
};

pub struct TimestampAdder {
    id: String,
}

impl TimestampAdder {
    pub fn new(id: &str) -> Self {
        Self { id: id.to_string() }
    }
}

#[async_trait]
impl<F> FrameProcessor<F> for TimestampAdder
where
    F: FrameProperties<u128> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        frame_data.set(&self.id, now_timestamp());
        Some(frame_data)
    }
}
