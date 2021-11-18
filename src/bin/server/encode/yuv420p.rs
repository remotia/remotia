#![allow(dead_code)]

use std::time::Instant;

use log::{debug, log_enabled};
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

        if log_enabled!(log::Level::Debug) {
            let conversion_start_time = Instant::now();
            raster::bgr_to_yuv_local_arrays(frame_buffer, &mut self.encoded_frame_buffer);
            debug!("YUV420P conversion time: {}", conversion_start_time.elapsed().as_millis());
        } else {
            raster::bgr_to_yuv_local_arrays(frame_buffer, &mut self.encoded_frame_buffer);
        }

        self.encoded_frame_buffer.len()
    }

    fn get_encoded_frame(&self) -> &[u8] {
        self.encoded_frame_buffer.as_slice()
    }
}
