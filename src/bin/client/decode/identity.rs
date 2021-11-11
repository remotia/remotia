use crate::error::ClientError;

use super::Decoder;

pub struct IdentityDecoder {
    decoded_frame_buffer: Vec<u8>
}

impl IdentityDecoder {
    pub fn new(width: usize, height: usize) -> Self {
        let frame_buffer_size = width * height * 3;

        IdentityDecoder {
            decoded_frame_buffer: vec![0 as u8; frame_buffer_size]
        }
    }
}

impl Decoder for IdentityDecoder {
    fn decode(&mut self, encoded_frame_buffer: &[u8]) -> Result<usize, ClientError> {
        self.decoded_frame_buffer.copy_from_slice(encoded_frame_buffer);

        Ok(0)
    }

    fn get_decoded_frame(&self) -> &[u8] {
        self.decoded_frame_buffer.as_slice()
    }
}

