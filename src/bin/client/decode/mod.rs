#![allow(dead_code)]

use crate::error::ClientError;

pub mod identity;
pub mod h264;

mod utils;
pub mod yuv420p;

pub trait Decoder {
    fn decode(&mut self, encoded_frame_buffer: &[u8]) -> Result<usize, ClientError>;
    fn get_decoded_frame(&self) -> &[u8];
}