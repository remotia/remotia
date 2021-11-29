#![allow(dead_code)]

use async_trait::async_trait;

use crate::client::error::ClientError;

// pub mod udp;
// pub mod tcp;
pub mod srt;

#[async_trait]
pub trait FrameReceiver {
    async fn receive_encoded_frame(&mut self, encoded_frame_buffer: & mut[u8]) -> Result<usize, ClientError>;
}