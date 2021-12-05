use std::{
    net::{SocketAddr, TcpStream, UdpSocket},
    time::Duration,
};

use crate::client::{decode::{
        h264::H264Decoder, h264rgb::H264RGBDecoder, h265::H265Decoder, identity::IdentityDecoder,
        yuv420p::YUV420PDecoder, Decoder,
    }, receive::{FrameReceiver, srt::SRTFrameReceiver, tcp::TCPFrameReceiver, udp::UDPFrameReceiver}};

pub fn setup_decoder_from_name(
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
        _ => panic!("Unknown decoder name"),
    };

    decoder
}

pub async fn setup_frame_receiver_by_name(
    server_address: SocketAddr,
    binding_port: &str,
    frame_receiver_name: &str,
) -> std::io::Result<Box<dyn FrameReceiver>> {
    match frame_receiver_name {
        "udp" => {
            let binding_address = format!("127.0.0.1:{}", binding_port);

            let socket = UdpSocket::bind(binding_address)?;
            socket
                .set_read_timeout(Some(Duration::from_millis(50)))
                .unwrap();

            let hello_buffer = [0; 16];
            socket.send_to(&hello_buffer, server_address).unwrap();

            Ok(Box::new(UDPFrameReceiver::create(socket, server_address)))
        }
        "tcp" => {
            let stream = TcpStream::connect(server_address)?;
            Ok(Box::new(TCPFrameReceiver::create(stream)))
        }
        "srt" => {
            Ok(Box::new(
                SRTFrameReceiver::new(
                    &server_address.to_string(),
                    Duration::from_millis(100),
                    Duration::from_millis(50),
                )
                .await
            ))
        }
        _ => panic!("Unknown frame receiver name"),
    }
}
