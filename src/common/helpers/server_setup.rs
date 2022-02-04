use std::{
    net::{TcpListener, UdpSocket},
    time::Duration,
};

use crate::server::{
    encode::{ffmpeg::h264::H264Encoder, Encoder},
    send::{
        remvsp::{RemVPSFrameSenderConfiguration, RemVSPFrameSender},
        srt::SRTFrameSender,
        tcp::TCPFrameSender,
        FrameSender,
    },
};
use log::info;

pub fn setup_encoder_by_name(
    width: usize,
    height: usize,
    encoder_name: &str,
) -> Box<dyn Encoder + Send> {
    info!("Setting up encoder...");

    let frame_size = width * height * 3;

    let encoder: Box<dyn Encoder + Send> = match encoder_name {
        "h264" => Box::new(H264Encoder::new(frame_size, width as i32, height as i32)),
        // "h264rgb" => Box::new(H264RGBEncoder::new(frame_size, width as i32, height as i32)),
        // "h265" => Box::new(H265Encoder::new(frame_size, width as i32, height as i32)),
        // "identity" => Box::new(IdentityEncoder::new()),
        _ => panic!("Unknown encoder name"),
    };

    encoder
}

pub async fn setup_frame_sender_by_name(
    frame_sender_name: &str,
) -> std::io::Result<Box<dyn FrameSender + Send>> {
    match frame_sender_name {
        "tcp" => {
            let listener = TcpListener::bind("127.0.0.1:5001")?;
            info!("Waiting for client connection...");
            let (stream, _client_address) = listener.accept()?;

            Ok(Box::new(TCPFrameSender::new(stream)))
        }
        "srt" => Ok(Box::new(
            SRTFrameSender::new(5001, Duration::from_millis(10), Duration::from_millis(50)).await,
        )),
        "remvsp" => Ok(Box::new(RemVSPFrameSender::listen(
            5001,
            512,
            RemVPSFrameSenderConfiguration {
                retransmission_frequency: 0.5,
            },
        ))),
        _ => panic!("Unknown frame sender"),
    }
}
