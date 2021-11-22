#![allow(dead_code)]

use super::Encoder;

pub struct IdentityEncoder {
    encoded_frame_buffer: Vec<u8>
}

impl IdentityEncoder {
    pub fn new(frame_buffer_size: usize) -> Self {
        IdentityEncoder {
            encoded_frame_buffer: vec![0 as u8; frame_buffer_size]
        }
    }
}

impl Encoder for IdentityEncoder {
    fn encode(&mut self, frame_buffer: &[u8]) -> usize {
        let encoded_frame_length = frame_buffer.len();

        self.encoded_frame_buffer.copy_from_slice(frame_buffer);

        encoded_frame_length
    }

    fn get_encoded_frame(&self) -> &[u8] {
        self.encoded_frame_buffer.as_slice()
    }
}
