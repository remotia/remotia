#![allow(dead_code)]

use std::time::Instant;

use async_trait::async_trait;

use crate::common::feedback::FeedbackMessage;

use crate::types::FrameData;

pub mod remvsp;
pub mod tcp;

#[async_trait]
pub trait FrameSender {
    async fn send_frame(&mut self, frame_data: &mut FrameData);
    fn handle_feedback(&mut self, message: FeedbackMessage);
}