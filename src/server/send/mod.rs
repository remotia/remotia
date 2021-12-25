#![allow(dead_code)]

use std::time::Instant;

use async_trait::async_trait;

pub mod remvsp;
pub mod tcp;
pub mod srt;
pub mod srt_manual_fragmentation;

#[async_trait]
pub trait FrameSender {
    async fn send_frame(&mut self, capture_timestamp: u128, frame_buffer: &[u8]) -> usize;
}