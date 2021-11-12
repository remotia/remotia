use crate::error::ClientError;

use super::{Decoder, utils::yuv2rgb::raster};

pub struct YUV420Decoder {
    decoded_frame_buffer: Vec<u8>
}

impl YUV420Decoder {
    pub fn new(width: usize, height: usize) -> Self {
        let frame_buffer_size = width * height * 3;

        YUV420Decoder {
            decoded_frame_buffer: vec![0 as u8; frame_buffer_size]
        }
    }
}

impl Decoder for YUV420Decoder {
    fn decode(&mut self, encoded_frame_buffer: &[u8]) -> Result<usize, ClientError> {
        raster::yuv_to_rgb(encoded_frame_buffer, &mut self.decoded_frame_buffer);

        Ok(self.decoded_frame_buffer.len())
    }

    fn get_decoded_frame(&self) -> &[u8] {
        self.decoded_frame_buffer.as_slice()
    }
}

