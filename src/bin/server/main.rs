extern crate scrap;

mod capture;

use scrap::{Capturer, Display, Frame};
use std::time::{Duration, Instant};
use std::thread;

use std::cmp::min;

// use std::net::UdpSocket;

use std::str::FromStr;
use std::net::{SocketAddr, SocketAddrV4};
use udt::*;

const PACKET_SIZE: usize = 512;

fn main() -> std::io::Result<()> {
    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");

    const FPS: u32 = 60;
    let spin_time = Duration::new(1, 0) / FPS;

    let fast_socket = UdtSocket::new(SocketFamily::AFInet, SocketType::Stream).unwrap();
    let client_ipv4_address = std::net::Ipv4Addr::from_str("127.0.0.1").unwrap();

    fast_socket.connect(SocketAddr::V4(SocketAddrV4::new(client_ipv4_address, 5001))).unwrap();

    loop {
        let loop_start_time = Instant::now();

        // Capture frame
        let result = capture::capture_frame(&mut capturer);

        let frame_buffer: Frame = match result {
            Ok(buffer) => buffer,
            Err(_error) => {
                thread::sleep(spin_time);
                continue;
            }
        };

        let transfer_start_time = Instant::now();

        fast_socket.send(&frame_buffer).unwrap();

        println!("Transfer time: {}", transfer_start_time.elapsed().as_millis());

        println!("Total time: {}", loop_start_time.elapsed().as_millis());
    }
}