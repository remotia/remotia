use log::info;
use scrap::Capturer;

use crate::encode::{
    ffmpeg::{
        h264::H264Encoder, h264rgb::H264RGBEncoder, h265::H265Encoder,
    },
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
        "h265" => Box::new(H265Encoder::new(frame_size, width as i32, height as i32)),
        "identity" => Box::new(IdentityEncoder::new(frame_size)),
        "yuv420p" => Box::new(YUV420PEncoder::new(width, height)),
        _ => panic!("Unknown encoder name"),
    };

    (packed_bgr_frame_buffer, encoder)
}

pub fn packed_bgra_to_packed_bgr(packed_bgra_buffer: &[u8], packed_bgr_buffer: &mut [u8]) {
    let pixels_count = packed_bgra_buffer.len() / 4;

    for i in 0..pixels_count {
        packed_bgr_buffer[i * 3] = packed_bgra_buffer[i * 4];
        packed_bgr_buffer[i * 3 + 1] = packed_bgra_buffer[i * 4 + 1];
        packed_bgr_buffer[i * 3 + 2] = packed_bgra_buffer[i * 4 + 2];
    }
}
