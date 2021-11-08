extern crate scrap;

mod capture;

use scrap::{Capturer, Display, Frame};
use std::time::{Duration, Instant};
use std::thread;

// use std::cmp::min;

use std::net::UdpSocket;

// use udt::*;

const PACKET_SIZE: usize = 512;

// const WIDTH: u32 = 1280;
// const HEIGHT: u32 = 720;

// const FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize) * 3;
// const SEND_BUFFER_SIZE: i32 = (FRAME_SIZE * 4) as i32;

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

    print!("Hello message received correctly. Streaming...");

    // let packet_buffer = [0, PACKET_SIZE];

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
        // let frame_buffer = vec![128; FRAME_SIZE];

        let transfer_start_time = Instant::now();

        let mut total_sent_bytes = 0;

        while total_sent_bytes < frame_buffer.len() {
            let packet_slice = &frame_buffer[total_sent_bytes..total_sent_bytes+PACKET_SIZE];

            let sent_bytes = socket.send_to(&packet_slice, &client_address)?;

            total_sent_bytes += sent_bytes;

            println!("Sent {}/{} bytes", total_sent_bytes, &frame_buffer.len());
        }

        println!("Transfer time: {}", transfer_start_time.elapsed().as_millis());

        println!("Total time: {}", loop_start_time.elapsed().as_millis());
    }
}