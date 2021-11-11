mod error;
mod receive;
mod decode;

use std::net::UdpSocket;

use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use beryllium::*;

// use decode::h264::H264Decoder;
use decode::identity::IdentityDecoder;
use pixels::{wgpu::Surface, Pixels, SurfaceTexture};
use receive::udp::UDPFrameReceiver;

use crate::decode::Decoder;
use crate::error::ClientError;
use crate::receive::FrameReceiver;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 72;

// const PACKET_SIZE: usize = 512;
const EXPECTED_FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize) * 3;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Init display
    let sdl = SDL::init(InitFlags::default())?;
    let window =
        sdl.create_raw_window("Remotia client", WindowPosition::Centered, WIDTH, HEIGHT, 0)?;

    let mut pixels = {
        let surface = Surface::create(&window);
        let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, surface);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    pixels.render()?;

    // Init socket
    let socket = UdpSocket::bind("127.0.0.1:5002")?;
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();

    let server_address = SocketAddr::from_str("127.0.0.1:5001")?;

    let hello_buffer = [0; 16];
    socket.send_to(&hello_buffer, server_address).unwrap();

    let frame_receiver = UDPFrameReceiver::create(&socket, &server_address);
    // let mut decoder = H264Decoder::new(WIDTH as usize, HEIGHT as usize);
    let mut decoder = IdentityDecoder::new(WIDTH as usize, HEIGHT as usize);

    let mut consecutive_connection_losses = 0;

    loop {
        println!(
            "Waiting for next frame (expected length: {})...",
            EXPECTED_FRAME_SIZE
        );

        // let canvas_buffer = pixels.get_frame();
        let mut encoded_frame_buffer = vec![0 as u8; EXPECTED_FRAME_SIZE];

        frame_receiver
            .receive_frame(&mut encoded_frame_buffer)
            .and_then(|_| {
                decoder.decode(&encoded_frame_buffer)
            }).and_then(|_| {
                let pixels_space = &mut pixels.get_frame()[..EXPECTED_FRAME_SIZE];
                pixels_space.copy_from_slice(decoder.get_decoded_frame());

                consecutive_connection_losses = 0;
                pixels.render().unwrap();
                println!("[SUCCESS] Frame rendered on screen");

                Ok(())
            }).unwrap_or_else(|e| {
                match e {
                    ClientError::InvalidWholeFrameHeader => consecutive_connection_losses = 0,
                    ClientError::H264SendPacketError => {
                        println!("H264 Send packet error")
                    }
                    _ => consecutive_connection_losses += 1,
                }

                println!(
                    "Error while receiving frame: {}, dropping (consecutive connection losses: {})",
                    e, consecutive_connection_losses
                );
            });

        if consecutive_connection_losses >= 10 {
            print!("Too much consecutive connection losses, closing stream");
            break;
        }
    }

    Ok(())
}
