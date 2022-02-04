use std::time::{Instant};

use serde::Serialize;

use crate::common::feedback::FeedbackMessage;

use async_trait::async_trait;

use super::{error::ServerError, types::ServerFrameData};

pub mod tcp;

pub mod console;

#[async_trait]
pub trait ServerProfiler {
    fn log_frame(&mut self, frame_data: ServerFrameData);
    async fn pull_feedback(&mut self) -> Option<FeedbackMessage>;
}
