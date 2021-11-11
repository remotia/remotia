#![allow(dead_code)]

use crate::error::ClientError;

pub mod udp;
pub mod tcp;

pub trait FrameReceiver {
    fn receive_frame(&mut self, frame_buffer: & mut[u8]) -> Result<(), ClientError>;
}