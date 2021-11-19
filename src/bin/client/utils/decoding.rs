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
