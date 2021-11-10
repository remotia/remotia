pub mod identity;

pub trait Encoder {
    fn encode(&mut self, frame_buffer: &[u8]) -> usize;
    fn get_encoded_frame(&self) -> &[u8];
}