use std::{cmp, fmt};

use async_trait::async_trait;
use log::debug;
use remotia_core::traits::{FrameProcessor, FrameProperties, FrameError};

pub struct TimestampBasedFrameDropper<T, E> {
    last_timestamp: T,
    error: E,
    stat_id: String,
}

impl<T: Default, E> TimestampBasedFrameDropper<T, E> {
    pub fn new(stat_id: &str, error: E) -> Self {
        Self {
            error,
            last_timestamp: Default::default(),
            stat_id: stat_id.to_string(),
        }
    }
}

#[async_trait]
impl<T, F, E> FrameProcessor<F> for TimestampBasedFrameDropper <T, E>  where
    F: FrameProperties<String, T> + FrameError<E> + Send + 'static,
    T: fmt::Debug + fmt::Display + cmp::PartialOrd + Send,
    E: Copy + Send
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let frame_timestamp = frame_data.get(&self.stat_id).unwrap();

        if frame_timestamp < self.last_timestamp {
            debug!(
                "Dropping frame with timestamp {} (last rendered timestamp: {})",
                frame_timestamp, self.last_timestamp
            );
            frame_data.report_error(self.error);
        } else {
            self.last_timestamp = frame_timestamp;
        }

        Some(frame_data)
    }
}
