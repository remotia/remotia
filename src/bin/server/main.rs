extern crate scrap;

mod capture;
mod send;

use std::time::{Duration, Instant};
use std::thread;

use std::net::{UdpSocket};

use libavif::{AvifData, Encoder, Error, RgbPixels, YuvFormat};
use scrap::{Capturer, Display, Frame};

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
    // let client_address = SocketAddr::from_str("127.0.0.1:5000").unwrap();

    println!("Hello message received correctly. Streaming...");

    socket.set_read_timeout(Some(Duration::from_millis(200))).unwrap();

    let frame_sender = FrameSender::create(&socket, PACKET_SIZE, &client_address);

    let width = capturer.width();
    let height = capturer.height();
    let frame_size = width * height * 3;

    loop {
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
        let encoded_frame = 
            encode_frame(
                width as u32, 
                height as u32, 
                frame_buffer)
        .unwrap();

        println!("Encoded frame size: {}/{}", encoded_frame.len(), frame_buffer.len());

        let transfer_start_time = Instant::now();

        frame_sender.send_frame(&encoded_frame);

        println!("Transfer time: {}", transfer_start_time.elapsed().as_millis());

        println!("Total time: {}", loop_start_time.elapsed().as_millis());
    }
}

fn encode_frame(width: u32, height: u32, rgb: &[u8]) -> Result<AvifData<'static>, Error> {
    let rgb = RgbPixels::new(width, height, rgb)?;
    let image = rgb.to_image(YuvFormat::Yuv444);


    let mut encoder = Encoder::new();
    encoder.set_max_threads(1);
    encoder.set_quantizer(200);
    encoder.set_speed(10);

    println!("Speed: {}", encoder.speed());

    encoder.encode(&image)
}