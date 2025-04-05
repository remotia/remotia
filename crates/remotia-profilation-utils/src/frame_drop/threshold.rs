use async_trait::async_trait;
use log::debug;
use remotia_core::traits::{FrameProcessor, FrameProperties, FrameError};

use std::{fmt, cmp};

pub struct ThresholdBasedFrameDropper<T, E> {
    threshold: T,
    error: E,
    stat_id: String,
}

impl<T, E> ThresholdBasedFrameDropper<T, E> {
    pub fn new(stat_id: &str, threshold: T, error: E) -> Self {
        Self {
            threshold,
            error,
            stat_id: stat_id.to_string(),
        }
    }
}

#[async_trait]
impl<T, F, E> FrameProcessor<F> for ThresholdBasedFrameDropper<T, E>  where
    F: FrameProperties<String, T> + FrameError<E> + Send + 'static,
    T: fmt::Debug + fmt::Display + cmp::PartialOrd + Send,
    E: Copy + Send
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let diff_value = frame_data.get(&self.stat_id).unwrap();

        if diff_value > self.threshold {
            debug!("Dropping frame due to higher than threshold value {} > {}", diff_value, self.threshold);
            frame_data.report_error(self.error);
        }

        Some(frame_data)
    }
}
