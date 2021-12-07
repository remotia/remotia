#![allow(dead_code)]

use std::{
    ffi::c_void,
    ptr::{null, NonNull},
};

use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::{AVDictionary, AVFrame},
    error::RsmpegError,
    ffi::{self, AVHWAccel},
};

use cstr::cstr;

use crate::server::encode::Encoder;

use super::{frame_builders::yuv420p::YUV420PAVFrameBuilder, FFMpegEncodingBridge};

pub struct H264VAAPIEncoder {
    encode_context: AVCodecContext,

    width: i32,
    height: i32,

    yuv420_avframe_builder: YUV420PAVFrameBuilder,
    ffmpeg_encoding_bridge: FFMpegEncodingBridge,
}

// TODO: Evaluate a safer way to move the encoder to another thread
// Necessary for multi-threaded pipelines
unsafe impl Send for H264VAAPIEncoder {}

impl H264VAAPIEncoder {
    pub fn new(frame_buffer_size: usize, width: i32, height: i32) -> Self {
        H264VAAPIEncoder {
            width: width,
            height: height,

            encode_context: {
                let encoder = AVCodec::find_encoder_by_name(cstr!("h264_vaapi")).unwrap();
                let mut encode_context = AVCodecContext::new(&encoder);

                encode_context.set_width(width);
                encode_context.set_height(height);
                encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 });
                encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
                encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_VAAPI);

                let mut encode_context = unsafe {
                    let raw_encode_context = encode_context.into_raw().as_ptr();

                    ffi::av_opt_set(
                        raw_encode_context as *mut c_void,
                        "vaapi_device".as_ptr() as *const i8,
                        "/dev/dri/renderD129".as_ptr() as *const i8,
                        0,
                    );

                    ffi::av_opt_set(
                        raw_encode_context as *mut c_void,
                        "vf".as_ptr() as *const i8,
                        "format=nv12,hwupload".as_ptr() as *const i8,
                        0,
                    );

                    AVCodecContext::from_raw(NonNull::new(raw_encode_context).unwrap())
                };

                let options = AVDictionary::new(cstr!(""), cstr!(""), 0);

                encode_context.open(Some(options)).unwrap();

                encode_context
            },

            yuv420_avframe_builder: YUV420PAVFrameBuilder::new(width as usize, height as usize),
            ffmpeg_encoding_bridge: FFMpegEncodingBridge::new(frame_buffer_size),
        }
    }
}

impl Encoder for H264VAAPIEncoder {
    fn encode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> usize {
        let avframe = self
            .yuv420_avframe_builder
            .create_avframe(&mut self.encode_context, input_buffer);

        self.ffmpeg_encoding_bridge
            .encode_avframe(&mut self.encode_context, avframe, output_buffer)
    }
}
