#![allow(dead_code)]

use std::time::Instant;

use async_trait::async_trait;

use crate::common::feedback::FeedbackMessage;

pub mod remvsp;
pub mod tcp;
pub mod srt;
pub mod srt_manual_fragmentation;

#[async_trait]
pub trait FrameSender {
    async fn send_frame(&mut self, capture_timestamp: u128, frame_buffer: &[u8]) -> usize;
    fn handle_feedback(&mut self, message: FeedbackMessage);
}