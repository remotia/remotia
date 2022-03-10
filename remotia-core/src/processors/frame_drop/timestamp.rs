use crate::{error::DropReason, traits::FrameProcessor, types::FrameData};
use async_trait::async_trait;
use log::debug;

pub struct TimestampBasedFrameDropper {
    last_timestamp: u128,
    stat_id: String,
}

impl TimestampBasedFrameDropper {
    pub fn new(stat_id: &str) -> Self {
        Self {
            last_timestamp: 0,
            stat_id: stat_id.to_string(),
        }
    }
}

#[async_trait]
impl FrameProcessor for TimestampBasedFrameDropper {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let frame_timestamp = frame_data.get(&self.stat_id);

        if frame_timestamp < self.last_timestamp {
            debug!(
                "Dropping frame with timestamp {} (last rendered timestamp: {})",
                frame_timestamp, self.last_timestamp
            );
            frame_data.set_drop_reason(Some(DropReason::StaleFrame));
        } else {
            self.last_timestamp = frame_timestamp;
        }

        Some(frame_data)
    }
}
