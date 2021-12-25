#![allow(dead_code)]

use crate::client::error::ClientError;

pub mod identity;
pub mod h264;
pub mod h264rgb;
pub mod h265;

mod utils;

pub trait Decoder {
    fn decode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> Result<usize, ClientError>;
}