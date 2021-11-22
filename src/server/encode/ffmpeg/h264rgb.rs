#![allow(dead_code)]

use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::{AVDictionary, AVFrame},
    error::RsmpegError,
    ffi,
};

use cstr::cstr;

use crate::server::encode::Encoder;

use super::{FFMpegEncodingBridge, frame_builders::{bgr::BGRAVFrameBuilder, yuv420p::YUV420PAVFrameBuilder}};

pub struct H264RGBEncoder {

    encode_context: AVCodecContext,

    width: i32,
    height: i32,

    bgr_avframe_builder: BGRAVFrameBuilder,
    ffmpeg_encoding_bridge: FFMpegEncodingBridge,
}

impl H264RGBEncoder {
    pub fn new(frame_buffer_size: usize, width: i32, height: i32) -> Self {
        H264RGBEncoder {
            width: width,
            height: height,

            encode_context: {
                let encoder = AVCodec::find_encoder_by_name(cstr!("libx264rgb")).unwrap();
                let mut encode_context = AVCodecContext::new(&encoder);

                encode_context.set_width(width);
                encode_context.set_height(height);
                encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 });
                encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
                encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_BGR24);

                let options = AVDictionary::new(cstr!("preset"), cstr!("ultrafast"), 0).set(
                    cstr!("tune"),
                    cstr!("zerolatency"),
                    0,
                );

                encode_context.open(Some(options)).unwrap();

                encode_context
            },

            bgr_avframe_builder: BGRAVFrameBuilder::new(),
            ffmpeg_encoding_bridge: FFMpegEncodingBridge::new(frame_buffer_size)
        }
    }
}

impl Encoder for H264RGBEncoder {
    fn encode(&mut self, frame_buffer: &[u8]) -> usize {
        let avframe = self
            .bgr_avframe_builder
            .create_avframe(&mut self.encode_context, frame_buffer);

        
        self.ffmpeg_encoding_bridge.encode_avframe(&mut self.encode_context, avframe)
    }

    fn get_encoded_frame(&self) -> &[u8] {
        self.ffmpeg_encoding_bridge.get_encoded_frame()
    }
}
