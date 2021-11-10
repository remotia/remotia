pub mod identity;

pub trait Decoder {
    fn decode(&mut self, encoded_frame_buffer: &[u8]);
    fn get_decoded_frame(&self) -> &[u8];
}