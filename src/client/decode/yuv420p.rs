use crate::client::error::ClientError;

use super::{Decoder, utils::yuv2bgr::raster};

pub struct YUV420PDecoder {
}

impl YUV420PDecoder {
    pub fn new(width: usize, height: usize) -> Self {
        Self { }
    }
}

impl Decoder for YUV420PDecoder {
    fn decode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> Result<usize, ClientError> {
        raster::yuv_to_bgr(input_buffer, output_buffer);

        Ok(self.decoded_frame_buffer.len())
    }
}

