use crate::decode::{Decoder, h264::H264Decoder, h264rgb::H264RGBDecoder, h265::H265Decoder, identity::IdentityDecoder, yuv420p::YUV420PDecoder};

pub fn setup_decoding_env(
    canvas_width: u32,
    canvas_height: u32,
    decoder_name: &str,
) -> Box<dyn Decoder> {
    let decoder: Box<dyn Decoder> = match decoder_name {
        "h264" => Box::new(H264Decoder::new(
            canvas_width as usize,
            canvas_height as usize,
        )),
        "h264rgb" => Box::new(H264RGBDecoder::new(
            canvas_width as usize,
            canvas_height as usize,
        )),
        "h265" => Box::new(H265Decoder::new(
            canvas_width as usize,
            canvas_height as usize,
        )),
        "identity" => Box::new(IdentityDecoder::new(
            canvas_width as usize,
            canvas_height as usize,
        )),
        "yuv420p" => Box::new(YUV420PDecoder::new(
            canvas_width as usize,
            canvas_height as usize,
        )),
        _ => panic!("Unknown decoder name")
    };

    decoder
}

pub fn packed_bgr_to_packed_rgba(packed_bgr_buffer: &[u8], packed_bgra_buffer: &mut [u8]) {
    let pixels_count = packed_bgra_buffer.len() / 4;

    for i in 0..pixels_count {
        packed_bgra_buffer[i * 4 + 2] = packed_bgr_buffer[i * 3];
        packed_bgra_buffer[i * 4 + 1] = packed_bgr_buffer[i * 3 + 1];
        packed_bgra_buffer[i * 4] = packed_bgr_buffer[i * 3 + 2];
    }
}