pub mod identity;
pub mod ffmpeg;
// pub mod yuv420p;

mod utils;

pub trait Encoder {
    fn encode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> usize;
    // fn get_encoded_frame(&self) -> &[u8];
}