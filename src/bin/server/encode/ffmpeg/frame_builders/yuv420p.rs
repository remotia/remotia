use log::debug;
use rsmpeg::{avcodec::AVCodecContext, avutil::AVFrame, error::RsmpegError};

use crate::encode::{Encoder, yuv420p::YUV420PEncoder};

pub struct YUV420PAVFrameBuilder {
    frame_count: i64,
    yuv420p_encoder: YUV420PEncoder,
}

impl YUV420PAVFrameBuilder {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            yuv420p_encoder: YUV420PEncoder::new(
                width,
                height,
            ),
            frame_count: 0,
        }
    }

    pub fn create_avframe(&mut self, encode_context: &mut AVCodecContext, frame_buffer: &[u8]) -> AVFrame {
        let mut avframe = AVFrame::new();
        avframe.set_format(encode_context.pix_fmt);
        avframe.set_width(encode_context.width);
        avframe.set_height(encode_context.height);
        avframe.set_pts(self.frame_count);
        avframe.alloc_buffer().unwrap();

        let data = avframe.data;
        let linesize = avframe.linesize;
        let width = encode_context.width as usize;
        let height = encode_context.height as usize;

        self.yuv420p_encoder.encode(frame_buffer);
        let yuv420p_frame_buffer = self.yuv420p_encoder.get_encoded_frame();

        let linesize_y = linesize[0] as usize;
        let linesize_cb = linesize[1] as usize;
        let linesize_cr = linesize[2] as usize;
        let y_data = unsafe { std::slice::from_raw_parts_mut(data[0], height * linesize_y) };
        let cb_data = unsafe { std::slice::from_raw_parts_mut(data[1], height / 2 * linesize_cb) };
        let cr_data = unsafe { std::slice::from_raw_parts_mut(data[2], height / 2 * linesize_cr) };

        let y_data_end_index = height * linesize_y;
        y_data.copy_from_slice(&yuv420p_frame_buffer[..y_data_end_index]);

        let cb_data_end_index = y_data_end_index + (height / 2) * linesize_cb;

        for y in 0..height / 2 {
            for x in 0..width / 2 {
                cb_data[y * linesize_cb + x] =
                    yuv420p_frame_buffer[y_data_end_index + y * linesize_cb + x];

                cr_data[y * linesize_cr + x] =
                    yuv420p_frame_buffer[cb_data_end_index + y * linesize_cr + x];
            }
        }

        debug!("Created avframe #{}", avframe.pts);

        self.frame_count += 1;

        avframe
    }
}

