use std::collections::VecDeque;

use crate::{error::DropReason, traits::FrameProcessor, types::FrameData, common::helpers::time::now_timestamp};
use async_trait::async_trait;
use log::debug;

pub struct TimestampBasedFrameReorderingBuffer {
    delay: u128,
    stat_id: String,

    last_release_timestamp: u128,
    held_frames: VecDeque<FrameData>,
}

impl TimestampBasedFrameReorderingBuffer {
    pub fn new(stat_id: &str, delay: u128) -> Self {
        Self {
            delay,
            stat_id: stat_id.to_string(),

            last_release_timestamp: 0,
            held_frames: VecDeque::new(),
        }
    }

    fn frame_stat(&self, frame_data: &FrameData) -> u128 {
        frame_data.get(&self.stat_id)
    }
}

#[async_trait]
impl FrameProcessor for TimestampBasedFrameReorderingBuffer {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let frame_timestamp = self.frame_stat(&frame_data);

        // Drop frame if its timestamp is after the last release
        if frame_timestamp < self.last_release_timestamp {
            debug!(
                "Dropping frame with timestamp {} (last released timestamp: {})",
                frame_timestamp, self.last_release_timestamp
            );
            frame_data.set_drop_reason(Some(DropReason::StaleFrame));
            return Some(frame_data);
        }

        // Held frame and order queue
        self.held_frames.push_back(frame_data);
        let stat_id = self.stat_id.clone();
        self.held_frames
            .make_contiguous()
            .sort_by(|a, b| {
                let a_stat = a.get(&stat_id);
                let b_stat = b.get(&stat_id);

                a_stat.partial_cmp(&b_stat).unwrap()
            });

        // Check if it's possible to release a frame
        let head = self.held_frames.get(0).unwrap();
        let head_diff = now_timestamp() - self.frame_stat(head);
        if head_diff >= self.delay {
            self.last_release_timestamp = frame_timestamp;
            return Some(self.held_frames.pop_front().unwrap());
        }

        None
    }
}
