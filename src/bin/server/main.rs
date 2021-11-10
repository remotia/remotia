extern crate scrap;

mod capture;
mod send;
mod encode;

use std::time::{Duration, Instant};
use std::thread::{self};

use std::net::{UdpSocket};

use scrap::{Capturer, Display, Frame};

use crate::encode::Encoder;
use crate::encode::identity::IdentityEncoder;
use crate::send::FrameSender;

const PACKET_SIZE: usize = 512;

fn main() -> std::io::Result<()> {
    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");

    const FPS: u32 = 60;
    let spin_time = Duration::new(1, 0) / FPS;

    let socket = UdpSocket::bind("127.0.0.1:5001")?;

    println!("Socket bound, waiting for hello message...");

    let mut hello_buffer = [0; 16];
    let (bytes_received, client_address) = socket.recv_from(&mut hello_buffer)?;
    assert_eq!(bytes_received, 16);
    // let client_address = SocketAddr::from_str("127.0.0.1:5000").unwrap();

    println!("Hello message received correctly. Streaming...");

    socket.set_read_timeout(Some(Duration::from_millis(200))).unwrap();

    let frame_sender = FrameSender::create(&socket, PACKET_SIZE, &client_address);

    let width = capturer.width();
    let height = capturer.height();
    let frame_size = width * height * 3;

    let mut encoder = IdentityEncoder::new(frame_size);

    loop {
        thread::sleep(spin_time);
        let loop_start_time = Instant::now();

        // Capture frame
        let result = capture::capture_frame(&mut capturer);

        println!("Frame captured");

        let frame_buffer: Frame = match result {
            Ok(buffer) => buffer,
            Err(_error) => {
                thread::sleep(spin_time);
                continue;
            }
        };

        println!("Encoding...");

        let frame_buffer = &frame_buffer[0..frame_size];
        let encoded_frame_length = encoder.encode(frame_buffer);

        println!("Encoded frame size: {}/{}", encoded_frame_length, frame_buffer.len());

        let transfer_start_time = Instant::now();

        frame_sender.send_frame(encoder.get_encoded_frame());

        println!("Transfer time: {}", transfer_start_time.elapsed().as_millis());

        println!("Total time: {}", loop_start_time.elapsed().as_millis());
    }
}
