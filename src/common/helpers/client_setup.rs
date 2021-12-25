use std::str::FromStr;
use std::{
    net::{SocketAddr, TcpStream, UdpSocket},
    time::Duration,
};

use crate::client::decode::h265::H265Decoder;
use crate::client::decode::identity::IdentityDecoder;
use crate::client::{
    decode::{h264::H264Decoder, h264rgb::H264RGBDecoder, Decoder},
    receive::{
        remvsp::RemVSPFrameReceiver, srt::SRTFrameReceiver, tcp::TCPFrameReceiver,
        FrameReceiver,
    },
};

pub fn setup_decoder_from_name(
    canvas_width: u32,
    canvas_height: u32,
    decoder_name: &str,
) -> Box<dyn Decoder + Send> {
    let decoder: Box<dyn Decoder + Send> = match decoder_name {
        "h264" => Box::new(H264Decoder::new()),
        "h264rgb" => Box::new(H264RGBDecoder::new()),
        "h265" => Box::new(H265Decoder::new()),
        "identity" => Box::new(IdentityDecoder::new(
            canvas_width as usize,
            canvas_height as usize,
        )),
        _ => panic!("Unknown decoder name"),
    };

    decoder
}

pub async fn setup_frame_receiver_by_name(
    server_address: SocketAddr,
    binding_port: &str,
    frame_receiver_name: &str,
) -> std::io::Result<Box<dyn FrameReceiver + Send>> {
    match frame_receiver_name {
        "tcp" => {
            let stream = TcpStream::connect(server_address)?;
            Ok(Box::new(TCPFrameReceiver::create(stream)))
        },
        "srt" => Ok(Box::new(
            SRTFrameReceiver::new(
                &server_address.to_string(),
                Duration::from_millis(10),
                Duration::from_millis(50),
            )
            .await,
        )),
        "remvsp" => Ok(Box::new(RemVSPFrameReceiver::connect(
            i16::from_str(binding_port).unwrap(),
            server_address,
        ).await)),
        _ => panic!("Unknown frame receiver name"),
    }
}
