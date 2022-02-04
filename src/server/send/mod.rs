#![allow(dead_code)]

use std::time::Instant;

use async_trait::async_trait;

use crate::common::feedback::FeedbackMessage;

use super::types::ServerFrameData;

pub mod remvsp;
pub mod tcp;
pub mod srt;

#[async_trait]
pub trait FrameSender {
    async fn send_frame(&mut self, frame_data: &mut ServerFrameData);
    fn handle_feedback(&mut self, message: FeedbackMessage);
}