#![allow(dead_code)]

use crate::error::ClientError;

pub mod udp;
pub mod tcp;

pub trait FrameReceiver {
    fn receive_encoded_frame(&mut self, encoded_frame_buffer: & mut[u8]) -> Result<usize, ClientError>;
}