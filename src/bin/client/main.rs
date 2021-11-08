// use std::net::UdpSocket;

use std::str::FromStr;
use std::net::{SocketAddr, SocketAddrV4};
use udt::*;

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
    let localhost = std::net::Ipv4Addr::from_str("127.0.0.1").unwrap();

    let sock = UdtSocket::new(SocketFamily::AFInet, SocketType::Datagram).unwrap();
    sock.bind(SocketAddr::V4(SocketAddrV4::new(localhost, 5001))).unwrap();
    let my_addr = sock.getsockname().unwrap();
    println!("Server bound to {:?}", my_addr);
    sock.listen(1).unwrap();
    let (connection_socket, peer) = sock.accept().unwrap();
    println!("Received new connection from peer {:?}", peer);

    // let mut packet_buffer: [u8; PACKET_SIZE] = [0; PACKET_SIZE];
    // let mut frame_buffer: [u8; FRAME_SIZE] = [0; FRAME_SIZE];

    connection_socket.setsockopt(UdtOpts::UDP_RCVBUF, RECV_BUFFER_SIZE).unwrap_err();
    connection_socket.setsockopt(UdtOpts::UDT_RCVBUF, RECV_BUFFER_SIZE).unwrap_err();

    loop {
        println!("Waiting for next frame (expected length: {})...", FRAME_SIZE);

        // let read_bytes = connection_socket.recv(pixels.get_frame(), FRAME_SIZE).unwrap();
        let read_bytes = connection_socket.recvmsg(pixels.get_frame()).unwrap();
        pixels.render()?;

        println!("Received a frame (read {} bytes)", read_bytes);
    }
}