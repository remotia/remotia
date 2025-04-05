use std::time::Duration;

use async_trait::async_trait;

use tokio::time::Interval;

use crate::{traits::FrameProcessor};

pub struct Ticker {
    interval: Interval,
}

impl Ticker {
    pub fn new(tick_interval: u64) -> Self {
        Self {
            interval: tokio::time::interval(Duration::from_millis(tick_interval)),
        }
    }
}

#[async_trait]
impl<F: Send + 'static> FrameProcessor<F> for Ticker {
    async fn process(&mut self, frame_data: F) -> Option<F> {
        self.interval.tick().await;
        Some(frame_data)
    }
}
