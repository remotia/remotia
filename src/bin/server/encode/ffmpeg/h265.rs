#![allow(dead_code)]

use log::debug;
use rgb2yuv420::convert_rgb_to_yuv420p;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::{AVDictionary, AVFrame},
    error::RsmpegError,
    ffi,
};

use cstr::cstr;

use crate::encode::{yuv420p::YUV420PEncoder, Encoder};

use super::{FFMpegEncodingBridge, YUV420PAVFrameBuilder};

pub struct H265Encoder {

    encode_context: AVCodecContext,

    width: i32,
    height: i32,

    yuv420_avframe_builder: YUV420PAVFrameBuilder,
    ffmpeg_encoding_bridge: FFMpegEncodingBridge,
}

impl H265Encoder {
    pub fn new(frame_buffer_size: usize, width: i32, height: i32) -> Self {
        H265Encoder {
            width: width,
            height: height,

            encode_context: {
                let encoder = AVCodec::find_encoder_by_name(cstr!("libx265")).unwrap();
                let mut encode_context = AVCodecContext::new(&encoder);

                encode_context.set_width(width);
                encode_context.set_height(height);
                encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 });
                encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
                encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);

                let options = AVDictionary::new(cstr!("preset"), cstr!("ultrafast"), 0).set(
                    cstr!("tune"),
                    cstr!("zerolatency"),
                    0,
                );

                encode_context.open(Some(options)).unwrap();

                encode_context
            },

            yuv420_avframe_builder: YUV420PAVFrameBuilder::new(width as usize, height as usize),
            ffmpeg_encoding_bridge: FFMpegEncodingBridge::new(frame_buffer_size)
        }
    }
}

impl Encoder for H265Encoder {
    fn encode(&mut self, frame_buffer: &[u8]) -> usize {
        let avframe = self
            .yuv420_avframe_builder
            .create_avframe(&mut self.encode_context, frame_buffer);

        
        self.ffmpeg_encoding_bridge.encode_avframe(&mut self.encode_context, avframe)
    }

    fn get_encoded_frame(&self) -> &[u8] {
        self.ffmpeg_encoding_bridge.get_encoded_frame()
    }
}
