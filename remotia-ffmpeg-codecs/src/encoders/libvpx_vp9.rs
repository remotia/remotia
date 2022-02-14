#![allow(dead_code)]

use std::{ptr::NonNull, time::Instant};

use log::debug;
use remotia::{traits::FrameProcessor, types::FrameData};
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::AVDictionary,
    ffi,
};

use async_trait::async_trait;

use cstr::cstr;

use super::{frame_builders::yuv420p::YUV420PAVFrameBuilder, FFMpegEncodingBridge};

pub struct LibVpxVP9Encoder {
    encode_context: AVCodecContext,

    width: i32,
    height: i32,

    yuv420_avframe_builder: YUV420PAVFrameBuilder,
    ffmpeg_encoding_bridge: FFMpegEncodingBridge,
}

// TODO: Evaluate a safer way to move the encoder to another thread
// Necessary for multi-threaded pipelines
unsafe impl Send for LibVpxVP9Encoder {}

impl LibVpxVP9Encoder {
    pub fn new(frame_buffer_size: usize, width: i32, height: i32) -> Self {
        let encode_context = init_encoder(width, height);

        LibVpxVP9Encoder {
            width,
            height,

            encode_context,

            yuv420_avframe_builder: YUV420PAVFrameBuilder::new(),
            ffmpeg_encoding_bridge: FFMpegEncodingBridge::new(frame_buffer_size),
        }
    }

    fn encode_on_frame_data(&mut self, frame_data: &mut FrameData) {
        let y_channel_buffer = frame_data
            .extract_writable_buffer("y_channel_buffer")
            .unwrap();
        let cb_channel_buffer = frame_data
            .extract_writable_buffer("cb_channel_buffer")
            .unwrap();
        let cr_channel_buffer = frame_data
            .extract_writable_buffer("cr_channel_buffer")
            .unwrap();

        let mut output_buffer = frame_data
            .extract_writable_buffer("encoded_frame_buffer")
            .expect("No encoded frame buffer in frame DTO");

        /*debug!(
            "[{}] Slice: {:?}",
            frame_data.get("capture_timestamp"),
            &input_buffer[0..64]
        );*/

        debug!("[{}]", frame_data.get("capture_timestamp"));

        let avframe_building_start_time = Instant::now();
        let avframe = self.yuv420_avframe_builder.create_avframe(
            &mut self.encode_context,
            &y_channel_buffer,
            &cb_channel_buffer,
            &cr_channel_buffer,
            false,
        );
        frame_data.set(
            "avframe_building_time",
            avframe_building_start_time.elapsed().as_millis(),
        );

        let encoded_bytes = self.ffmpeg_encoding_bridge.encode_avframe(
            &mut self.encode_context,
            avframe,
            &mut output_buffer,
        );

        frame_data.insert_writable_buffer("y_channel_buffer", y_channel_buffer);
        frame_data.insert_writable_buffer("cb_channel_buffer", cb_channel_buffer);
        frame_data.insert_writable_buffer("cr_channel_buffer", cr_channel_buffer);

        frame_data.insert_writable_buffer("encoded_frame_buffer", output_buffer);

        frame_data.set("encoded_size", encoded_bytes as u128);
    }
}

fn init_encoder(width: i32, height: i32) -> AVCodecContext {
    let encoder = AVCodec::find_encoder_by_name(cstr!("libvpx-vp9")).unwrap();
    let mut encode_context = AVCodecContext::new(&encoder);
    encode_context.set_width(width);
    encode_context.set_height(height);
    encode_context.set_time_base(ffi::AVRational { num: 1, den: 60 });
    encode_context.set_framerate(ffi::AVRational { num: 60, den: 1 });
    encode_context.set_pix_fmt(rsmpeg::ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);
    let mut encode_context = unsafe {
        let raw_encode_context = encode_context.into_raw().as_ptr();
        AVCodecContext::from_raw(NonNull::new(raw_encode_context).unwrap())
    };

    let options = AVDictionary::new(cstr!(""), cstr!(""), 0)
        .set(cstr!("quality"), cstr!("realtime"), 0)
        .set(cstr!("g"), cstr!("1"), 0)
        .set(cstr!("lossless"), cstr!("1"), 0)
        .set(cstr!("speed"), cstr!("6"), 0);

    encode_context.open(Some(options)).unwrap();
    encode_context
}

#[async_trait]
impl FrameProcessor for LibVpxVP9Encoder {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        self.encode_on_frame_data(&mut frame_data);
        Some(frame_data)
    }
}
