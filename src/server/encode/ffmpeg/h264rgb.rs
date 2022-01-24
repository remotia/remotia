#![allow(dead_code)]

use std::ptr::NonNull;

use log::debug;
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::{AVDictionary, AVFrame},
    error::RsmpegError,
    ffi,
};

use cstr::cstr;

use crate::{common::feedback::FeedbackMessage, server::encode::Encoder};

use super::{
    frame_builders::{bgr::BGRAVFrameBuilder, yuv420p::YUV420PAVFrameBuilder},
    FFMpegEncodingBridge,
};

#[derive(Default, Debug)]
pub struct H264RGBEncoderState {
    encoded_frames: usize,
}

pub struct H264RGBEncoder {
    encode_context: AVCodecContext,

    width: i32,
    height: i32,

    state: H264RGBEncoderState,

    bgr_avframe_builder: BGRAVFrameBuilder,
    ffmpeg_encoding_bridge: FFMpegEncodingBridge,
}

// TODO: Evaluate a safer way to move the encoder to another thread
// Necessary for multi-threaded pipelines
unsafe impl Send for H264RGBEncoder {}

impl H264RGBEncoder {
    pub fn new(frame_buffer_size: usize, width: i32, height: i32) -> Self {
        H264RGBEncoder {
            width: width,
            height: height,

            state: H264RGBEncoderState::default(),

            encode_context: {
                let encoder = AVCodec::find_encoder_by_name(cstr!("libx264rgb")).unwrap();
                let mut encode_context = AVCodecContext::new(&encoder);

                encode_context.set_width(width);
                encode_context.set_height(height);
                encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 });
                encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
                encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_BGR24);

                let mut encode_context = unsafe {
                    let raw_encode_context = encode_context.into_raw().as_ptr();

                    // (*raw_encode_context).bit_rate = 20000 * 1000;

                    AVCodecContext::from_raw(NonNull::new(raw_encode_context).unwrap())
                };

                let options = AVDictionary::new(cstr!(""), cstr!(""), 0)
                    .set(cstr!("preset"), cstr!("ultrafast"), 0)
                    .set(cstr!("tune"), cstr!("zerolatency"), 0);

                encode_context.open(Some(options)).unwrap();

                encode_context
            },

            bgr_avframe_builder: BGRAVFrameBuilder::new(),
            ffmpeg_encoding_bridge: FFMpegEncodingBridge::new(frame_buffer_size),
        }
    }
}

impl Encoder for H264RGBEncoder {
    fn encode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> usize {
        let key_frame = self.state.encoded_frames % 4 == 0;

        let avframe = self.bgr_avframe_builder.create_avframe(
            &mut self.encode_context,
            input_buffer,
            key_frame,
        );

        let encoded_bytes = self.ffmpeg_encoding_bridge.encode_avframe(
            &mut self.encode_context,
            avframe,
            output_buffer,
        );

        self.state.encoded_frames += 1;

        encoded_bytes
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
