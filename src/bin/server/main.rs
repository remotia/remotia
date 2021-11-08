extern crate scrap;

mod capture;
mod send;

use scrap::{Capturer, Display, Frame};
use std::time::{Duration, Instant};
use std::thread;

use std::net::UdpSocket;

use crate::send::FrameSender;

const PACKET_SIZE: usize = 512;

fn main() -> std::io::Result<()> {
    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");

    const FPS: u32 = 24;
    let spin_time = Duration::new(1, 0) / FPS;

    let socket = UdpSocket::bind("127.0.0.1:5001")?;

    println!("Socket bound, waiting for hello message...");

    let mut hello_buffer = [0; 16];
    let (bytes_received, client_address) = socket.recv_from(&mut hello_buffer)?;

    assert_eq!(bytes_received, 16);

    print!("Hello message received correctly. Streaming...");

    socket.set_read_timeout(Some(Duration::from_millis(200))).unwrap();

    let frame_sender = FrameSender::create(&socket, PACKET_SIZE, &client_address);

    let frame_size = capturer.width() * capturer.height() * 3;

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

        frame_sender.send_frame(&frame_buffer[0..frame_size]);

        println!("Transfer time: {}", transfer_start_time.elapsed().as_millis());

        println!("Total time: {}", loop_start_time.elapsed().as_millis());
    }
}