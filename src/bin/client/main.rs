#![allow(unused_imports)]

mod decode;
mod error;
mod receive;

use std::env;
use std::net::TcpStream;

use std::net::SocketAddr;
use std::net::UdpSocket;
use std::str::FromStr;
use std::time::Duration;

use beryllium::*;

use decode::h264::H264Decoder;
use decode::identity::IdentityDecoder;
use log::info;
use log::{debug, error, warn};
use pixels::wgpu;
use pixels::PixelsBuilder;
use pixels::{wgpu::Surface, Pixels, SurfaceTexture};
use receive::tcp::TCPFrameReceiver;
use zstring::zstr;

use crate::decode::h265::H265Decoder;
use crate::decode::yuv420p::YUV420PDecoder;
use crate::decode::Decoder;
use crate::error::ClientError;
use crate::receive::udp::UDPFrameReceiver;
use crate::receive::FrameReceiver;

#[allow(dead_code)]
fn enstablish_udp_connection(server_address: &SocketAddr) -> std::io::Result<UdpSocket> {
    let socket = UdpSocket::bind("127.0.0.1:5002")?;
    socket
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();

    let hello_buffer = [0; 16];
    socket.send_to(&hello_buffer, server_address).unwrap();

    Ok(socket)
}

fn parse_canvas_resolution_arg(arg: &String) -> (u32, u32) {
    let canvas_resolution_split: Vec<&str> = arg.split("x").collect();

    let width_str = canvas_resolution_split[0];
    let height_str = canvas_resolution_split[1];

    let canvas_width: u32 = u32::from_str(width_str).unwrap_or_else(|e| {
        panic!(
            "Unable to parse width '{}': {}",
            width_str, e
        )
    });

    let canvas_height: u32 = u32::from_str(height_str).unwrap_or_else(|e| {
        panic!(
            "Unable to parse height '{}': {}",
            height_str, e
        )
    });


    (canvas_width, canvas_height)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    let (canvas_width, canvas_height) = parse_canvas_resolution_arg(&args[1]);
    let expected_frame_size: usize = (canvas_width as usize) * (canvas_height as usize) * 3;

    // Init display
    let sdl = SDL::init(InitFlags::default())?;
    let window = sdl.create_raw_window(
        "Remotia client",
        WindowPosition::Centered,
        canvas_width,
        canvas_height,
        0,
    )?;

    let mut pixels = {
        // let surface = Surface::create(&window);
        let surface_texture = SurfaceTexture::new(canvas_width, canvas_height, &window);
        PixelsBuilder::new(canvas_width, canvas_height, surface_texture)
            // .texture_format(wgpu::TextureFormat::Bgra8Unorm)
            .build()?
        // Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    pixels.render()?;

    let server_address = SocketAddr::from_str("127.0.0.1:5001")?;

    /*let socket = enstablish_udp_connection(&server_address)?;
    let mut frame_receiver = UDPFrameReceiver::create(&socket, &server_address);*/

    let mut stream = TcpStream::connect(server_address)?;
    let mut frame_receiver = TCPFrameReceiver::create(&mut stream);

    let mut decoder = setup_decoding_env(canvas_width, canvas_height);

    let mut consecutive_connection_losses = 0;

    let mut encoded_frame_buffer = vec![0 as u8; expected_frame_size];

    info!("Starting to receive stream...");

    loop {
        debug!("Waiting for next frame...");

        // let canvas_buffer = pixels.get_frame();

        frame_receiver
            .receive_encoded_frame(&mut encoded_frame_buffer)
            .and_then(|received_data_length| {
                debug!("Decoding {} received bytes", received_data_length);
                decoder.decode(&encoded_frame_buffer[..received_data_length])?;

                Ok(decoder.get_decoded_frame())
            })
            .and_then(|decoded_frame| {
                packed_bgr_to_packed_rgba(decoded_frame, pixels.get_frame());

                consecutive_connection_losses = 0;
                pixels.render().unwrap();
                debug!("[SUCCESS] Frame rendered on screen");

                Ok(())
            })
            .unwrap_or_else(|e| {
                match e {
                    ClientError::InvalidWholeFrameHeader => consecutive_connection_losses = 0,
                    ClientError::H264SendPacketError => {
                        debug!("H264 Send packet error")
                    }
                    _ => consecutive_connection_losses += 1,
                }

                warn!(
                    "Error while receiving frame: {}, dropping (consecutive connection losses: {})",
                    e, consecutive_connection_losses
                );
            });

        if consecutive_connection_losses >= 100 {
            error!("Too much consecutive connection losses, closing stream");
            break;
        }
    }

    Ok(())
}

fn setup_decoding_env(canvas_width: u32, canvas_height: u32) -> Box<dyn Decoder> {
    // let decoder = H264Decoder::new(canvas_width as usize, canvas_height as usize);
    // let decoder = IdentityDecoder::new(canvas_width as usize, canvas_height as usize);
    let decoder = YUV420PDecoder::new(canvas_width as usize, canvas_height as usize);

    Box::new(decoder)
}

fn packed_bgr_to_packed_rgba(packed_bgr_buffer: &[u8], packed_bgra_buffer: &mut [u8]) {
    let pixels_count = packed_bgra_buffer.len() / 4;

    for i in 0..pixels_count {
        packed_bgra_buffer[i * 4 + 2] = packed_bgr_buffer[i * 3];
        packed_bgra_buffer[i * 4 + 1] = packed_bgr_buffer[i * 3 + 1];
        packed_bgra_buffer[i * 4] = packed_bgr_buffer[i * 3 + 2];
    }
}
