use std::net::UdpSocket;

use std::str::FromStr;
use std::net::{SocketAddr};

use beryllium::*;
use pixels::{wgpu::Surface, Pixels, SurfaceTexture};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

// const PACKET_SIZE: usize = 512;
const FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize) * 3;
const RECV_BUFFER_SIZE: i32 = (FRAME_SIZE * 4) as i32;

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

    let server_address = SocketAddr::from_str("127.0.0.1:5001")?;

    let hello_buffer = [0; 16];
    socket.send_to(&hello_buffer, server_address).unwrap();

    // let mut packet_buffer: [u8; PACKET_SIZE] = [0; PACKET_SIZE];
    // let mut frame_buffer: [u8; FRAME_SIZE] = [0; FRAME_SIZE];

    loop {
        println!("Waiting for next frame (expected length: {})...", FRAME_SIZE);

        let frame_buffer = pixels.get_frame();

        let mut total_received_bytes = 0;

        while total_received_bytes < frame_buffer.len() {
            let packet_slice = &mut frame_buffer[total_received_bytes..];

            let received_bytes = socket.recv(packet_slice)?;

            total_received_bytes += received_bytes;

            println!("Received {}/{} bytes", total_received_bytes, &frame_buffer.len());
        }

        pixels.render()?;

        println!("Received a frame (received {} bytes)", total_received_bytes);
    }
}