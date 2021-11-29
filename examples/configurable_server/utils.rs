use std::{net::{TcpListener, UdpSocket}, time::Duration};

use log::info;
use remotia::server::{encode::{Encoder, ffmpeg::{h264::H264Encoder, h264rgb::H264RGBEncoder, h265::H265Encoder}, identity::IdentityEncoder, yuv420p::YUV420PEncoder}, send::{FrameSender, tcp::TCPFrameSender, udp::UDPFrameSender}};

pub fn setup_encoder_by_name(width: usize, height: usize, encoder_name: &str) -> Box<dyn Encoder> {
    info!("Setting up encoder...");

    let frame_size = width * height * 3;

    let encoder: Box<dyn Encoder> = match encoder_name {
        "h264" => Box::new(H264Encoder::new(frame_size, width as i32, height as i32)),
        "h264rgb" => Box::new(H264RGBEncoder::new(frame_size, width as i32, height as i32)),
        "h265" => Box::new(H265Encoder::new(frame_size, width as i32, height as i32)),
        "identity" => Box::new(IdentityEncoder::new(frame_size)),
        "yuv420p" => Box::new(YUV420PEncoder::new(width, height)),
        _ => panic!("Unknown encoder name"),
    };

    encoder
}

pub fn setup_frame_sender_by_name(frame_sender_name: &str) -> std::io::Result<Box<dyn FrameSender>> {
    match frame_sender_name {
        "udp" => {
            const PACKET_SIZE: usize = 512;
            let socket = UdpSocket::bind("127.0.0.1:5001")?;

            info!("Socket bound, waiting for hello message...");

            let mut hello_buffer = [0; 16];
            let (bytes_received, client_address) = socket.recv_from(&mut hello_buffer)?;
            assert_eq!(bytes_received, 16);

            info!("Hello message received correctly. Streaming...");
            socket
                .set_read_timeout(Some(Duration::from_millis(200)))
                .unwrap();

            Ok(Box::new(UDPFrameSender::new(socket, PACKET_SIZE, client_address)))
        },
        "tcp" => {
            let listener = TcpListener::bind("127.0.0.1:5001")?;
            info!("Waiting for client connection...");
            let (stream, _client_address) = listener.accept()?;

            Ok(Box::new(TCPFrameSender::new(stream)))
        },
        _ => panic!("Unknown frame sender")
    }
}