#![allow(dead_code)]

use log::debug;
use rgb2yuv420::convert_rgb_to_yuv420p;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::AVFrame,
    error::RsmpegError,
    ffi,
};

use cstr::cstr;

use super::{Encoder, yuv420p::YUV420PEncoder};

pub struct H264Encoder {
    encoded_frame_buffer: Vec<u8>,
    encoded_frame_length: usize,

    encode_context: AVCodecContext,

    // output_context: ffi::AVFormatContext,
    width: i32,
    height: i32,

    frame_count: i64,

    yuv420p_encoder: YUV420PEncoder
}

impl H264Encoder {
    pub fn new(frame_buffer_size: usize, width: i32, height: i32) -> Self {
        H264Encoder {
            encoded_frame_buffer: vec![0 as u8; frame_buffer_size],
            encoded_frame_length: 0,
            width: width,
            height: height,

            encode_context: {
                let encoder = AVCodec::find_encoder_by_name(cstr!("libx264")).unwrap();
                let mut encode_context = AVCodecContext::new(&encoder);
                encode_context.set_bit_rate(400000);
                encode_context.set_width(width);
                encode_context.set_height(height);
                encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 });
                encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
                encode_context.set_gop_size(10);
                encode_context.set_max_b_frames(1);
                encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);
                encode_context.open(None).unwrap();

                encode_context
            },
            
            frame_count: 0,

            yuv420p_encoder: YUV420PEncoder::new(width as usize, height as usize)
        }
    }

    fn create_avframe(&mut self, frame_buffer: &[u8]) -> AVFrame {
        let mut avframe = AVFrame::new();
        avframe.set_format(self.encode_context.pix_fmt);
        avframe.set_width(self.encode_context.width);
        avframe.set_height(self.encode_context.height);
        avframe.set_pts(self.frame_count);
        avframe.alloc_buffer().unwrap();

        let data = avframe.data;
        let linesize = avframe.linesize;
        let width = self.width as usize;
        let height = self.height as usize;

        self.yuv420p_encoder.encode(frame_buffer);
        let yuv420p_frame_buffer = self.yuv420p_encoder.get_encoded_frame();
            // convert_rgb_to_yuv420p(frame_buffer, width as u32, height as u32, 3);

        let linesize_y = linesize[0] as usize;
        let linesize_cb = linesize[1] as usize;
        let linesize_cr = linesize[2] as usize;
        let y_data = unsafe { std::slice::from_raw_parts_mut(
            data[0], height * linesize_y) };
        let cb_data = unsafe { std::slice::from_raw_parts_mut(
            data[1], height / 2 * linesize_cb) };
        let cr_data = unsafe { std::slice::from_raw_parts_mut(
            data[2], height / 2 * linesize_cr) };

        // debug!("Sizes: {} {}", frame_buffer.len(), yuv420_frame_buffer.len());

        // prepare a dummy image
        let y_data_end_index = height * linesize_y;
        y_data.copy_from_slice(&yuv420p_frame_buffer[..y_data_end_index]);

        let cb_data_end_index = y_data_end_index + (height/2) * linesize_cb;

        /*debug!("Y end index: {} (linesize {})", y_data_end_index, linesize_y);
        debug!("Cb end index: {} (linesize {})", cb_data_end_index, linesize_cb);
        debug!("Cr linesize: {}", linesize_cr);*/

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

impl Encoder for H264Encoder {
    fn encode(&mut self, frame_buffer: &[u8]) -> usize {
        self.encoded_frame_length = 0;

        let avframe = self.create_avframe(frame_buffer);

        self.encode_context.send_frame(Some(&avframe)).unwrap();

        loop {
            let packet = match self.encode_context.receive_packet() {
                Ok(packet) => {
                    // debug!("Received packet of size {}", packet.size);
                    packet
                }
                Err(RsmpegError::EncoderDrainError) => {
                    debug!("Drain error, breaking the loop");
                    break;
                }
                Err(RsmpegError::EncoderFlushedError) => {
                    debug!("Flushed error, breaking the loop");
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };

            let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };

            let start_index = self.encoded_frame_length;
            let end_index = self.encoded_frame_length + data.len();

            self.encoded_frame_buffer[start_index..end_index].copy_from_slice(data);

            self.encoded_frame_length = end_index;
        }

        self.encoded_frame_length as usize
    }

    fn get_encoded_frame(&self) -> &[u8] {
        &self.encoded_frame_buffer.as_slice()[..self.encoded_frame_length]
    }
}
