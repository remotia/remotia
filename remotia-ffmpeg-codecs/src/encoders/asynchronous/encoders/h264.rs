#![allow(dead_code)]

use std::{
    ffi::CString,
    ptr::NonNull,
    sync::{Arc, Mutex}, time::Instant,
};

use remotia::{error::DropReason, traits::FrameProcessor, types::FrameData};
use rsmpeg::{
    avcodec::{AVCodec, AVCodecContext},
    avutil::AVDictionary,
    ffi,
};

use async_trait::async_trait;

use cstr::cstr;

use crate::encoders::frame_builders::yuv420p::YUV420PAVFrameBuilder;

pub struct AsyncH264Encoder {
    encode_context: Arc<Mutex<AVCodecContext>>,
    width: i32,
    height: i32,
}

unsafe impl Send for AsyncH264Encoder {}

impl AsyncH264Encoder {
    pub fn new(width: i32, height: i32) -> Self {
        AsyncH264Encoder {
            encode_context: Arc::new(Mutex::new(init_encoder(width, height, 21))),
            width,
            height,
        }
    }

    pub fn pusher(&self) -> AsyncH264EncoderFramePusher {
        AsyncH264EncoderFramePusher::new(self.encode_context.clone())
    }

    pub fn puller(&self) -> AsyncH264EncoderPacketPuller {
        AsyncH264EncoderPacketPuller::new(self.encode_context.clone())
    }
}

fn init_encoder(width: i32, height: i32, crf: u32) -> AVCodecContext {
    let encoder = AVCodec::find_encoder_by_name(cstr!("libx264")).unwrap();
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

    let crf_str = format!("{}", crf);
    let crf_str = CString::new(crf_str).unwrap();

    let options = AVDictionary::new(cstr!(""), cstr!(""), 0)
        .set(cstr!("preset"), cstr!("ultrafast"), 0)
        .set(cstr!("crf"), &crf_str, 0)
        .set(cstr!("threads"), cstr!("0"), 0)
        .set(cstr!("thread_type"), cstr!("frame"), 0)
        .set(cstr!("tune"), cstr!("zerolatency"), 0);

    encode_context.open(Some(options)).unwrap();
    encode_context
}

pub struct AsyncH264EncoderFramePusher {
    encode_context: Arc<Mutex<AVCodecContext>>,
    yuv420_avframe_builder: YUV420PAVFrameBuilder,
}
unsafe impl Send for AsyncH264EncoderFramePusher {}

impl AsyncH264EncoderFramePusher {
    pub fn new(encode_context: Arc<Mutex<AVCodecContext>>) -> Self {
        Self {
            encode_context,
            yuv420_avframe_builder: YUV420PAVFrameBuilder::new(),
        }
    }
}

#[async_trait]
impl FrameProcessor for AsyncH264EncoderFramePusher {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let push_mutex_lock_start = Instant::now();
        let mut encode_context = self.encode_context.lock().unwrap();
        frame_data.set("push_mutex_lock_time", push_mutex_lock_start.elapsed().as_millis());

        let input_buffer = frame_data
            .get_writable_buffer_ref("raw_frame_buffer")
            .expect("No raw frame buffer in frame DTO");

        let avframe_creation_start = Instant::now();
        let avframe =
            self.yuv420_avframe_builder
                .create_avframe(&mut encode_context, &input_buffer, false);
        frame_data.set("avframe_creation_time", avframe_creation_start.elapsed().as_millis());

        let send_frame_start = Instant::now();
        if let Err(_) = encode_context.send_frame(Some(&avframe)) {
            frame_data.set_drop_reason(Some(DropReason::CodecError));
        }
        frame_data.set("send_frame_time", send_frame_start.elapsed().as_millis());

        Some(frame_data)
    }
}

pub struct AsyncH264EncoderPacketPuller {
    encode_context: Arc<Mutex<AVCodecContext>>,
}
unsafe impl Send for AsyncH264EncoderPacketPuller {}

impl AsyncH264EncoderPacketPuller {
    pub fn new(encode_context: Arc<Mutex<AVCodecContext>>) -> Self {
        Self { encode_context }
    }
}

#[async_trait]
impl FrameProcessor for AsyncH264EncoderPacketPuller {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let pull_mutex_lock_start = Instant::now();
        let mut encode_context = self.encode_context.lock().unwrap();
        frame_data.set("pull_mutex_lock_time", pull_mutex_lock_start.elapsed().as_millis());

        let packet = match encode_context.receive_packet() {
            Ok(packet) => packet,
            Err(_) => {
                frame_data.set_drop_reason(Some(DropReason::CodecError));
                return Some(frame_data);
            }
        };

        let data = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };

        let output_buffer = frame_data
            .get_writable_buffer_ref("encoded_frame_buffer")
            .expect("No encoded frame buffer in frame DTO");

        (&mut output_buffer[..data.len()]).copy_from_slice(data);
        frame_data.set("encoded_size", data.len() as u128);

        Some(frame_data)
    }
}
