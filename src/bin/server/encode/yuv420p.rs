#![allow(dead_code)]

use rgb2yuv420::convert_rgb_to_yuv420p;

use super::{Encoder, utils::bgr2yuv::raster};

pub struct YUV420PEncoder {
    encoded_frame_buffer: Vec<u8>
}

impl YUV420PEncoder {
    pub fn new(width: usize, height: usize) -> Self {
        YUV420PEncoder {
            encoded_frame_buffer: vec![0 as u8; (width * height * 3) / 2]
        }
    }
}

impl Encoder for YUV420PEncoder {
    fn encode(&mut self, frame_buffer: &[u8]) -> usize {
        self.encoded_frame_buffer.fill(0);

        raster::bgr_to_yuv(frame_buffer, &mut self.encoded_frame_buffer);

        self.encoded_frame_buffer.len()
    }

    fn get_encoded_frame(&self) -> &[u8] {
        self.encoded_frame_buffer.as_slice()
    }
}
