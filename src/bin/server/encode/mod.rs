pub mod identity;
pub mod ffmpeg;
pub mod yuv420p;

mod utils;

pub trait Encoder {
    fn encode(&mut self, frame_buffer: &[u8]) -> usize;
    fn get_encoded_frame(&self) -> &[u8];
}