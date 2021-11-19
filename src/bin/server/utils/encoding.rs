use log::info;
use scrap::Capturer;

use crate::encode::{
    ffmpeg::{h264::H264Encoder, h264rgb::H264RGBEncoder},
    identity::IdentityEncoder,
    yuv420p::YUV420PEncoder,
    Encoder,
};

pub fn setup_encoding_env(capturer: &Capturer, encoder_name: &str) -> (Vec<u8>, Box<dyn Encoder>) {
    info!("Setting up encoder...");

    let width = capturer.width();
    let height = capturer.height();
    let frame_size = width * height * 3;
    let packed_bgr_frame_buffer: Vec<u8> = vec![0; frame_size];

    let encoder: Box<dyn Encoder> = match encoder_name {
        "h264" => Box::new(H264Encoder::new(frame_size, width as i32, height as i32)),
        "h264rgb" => Box::new(H264RGBEncoder::new(frame_size, width as i32, height as i32)),
        "identity" => Box::new(IdentityEncoder::new(frame_size)),
        "yuv420p" => Box::new(YUV420PEncoder::new(width, height)),
        _ => panic!("Unknown encoder name"),
    };

    (packed_bgr_frame_buffer, encoder)
}
