pub mod identity;
pub mod ffmpeg;

mod utils;

pub trait Encoder {
    fn encode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> usize;
}