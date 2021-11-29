#![allow(dead_code)]

use async_trait::async_trait;

// pub mod udp;
// pub mod tcp;
pub mod srt;

#[async_trait]
pub trait FrameSender {
    async fn send_frame(&mut self, frame_buffer: &[u8]);
}