mod receive;

use std::net::UdpSocket;

use std::str::FromStr;
use std::net::{SocketAddr};
use std::time::Duration;

use beryllium::*;
use pixels::{wgpu::Surface, Pixels, SurfaceTexture};
use receive::FrameReceiver;

const WIDTH: u32 = 128;
const HEIGHT: u32 = 72;

// const PACKET_SIZE: usize = 512;
const FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize) * 3;

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
    socket.set_read_timeout(Some(Duration::from_secs(1))).unwrap();

    let server_address = SocketAddr::from_str("127.0.0.1:5001")?;

    let hello_buffer = [0; 16];
    socket.send_to(&hello_buffer, server_address).unwrap();

    let frame_receiver = FrameReceiver::create(&socket, &server_address);

    let mut consecutive_dropped_frames = 0;

    loop {
        println!("Waiting for next frame (expected length: {})...", FRAME_SIZE);

        match frame_receiver.receive_frame(pixels.get_frame()) {
            Ok(_) => {
                consecutive_dropped_frames = 0;
                pixels.render()?;
            },
            Err(_) => {
                println!("Error while receiving frame, dropping");
                consecutive_dropped_frames += 1;
            }
        };

        if consecutive_dropped_frames >= 5 {
            print!("Too much consecutive dropped frames, closing stream");
            break;
        }
    };

    Ok(())
}
