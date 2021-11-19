#![allow(unused_imports)]
#![feature(test)]

extern crate scrap;

mod capture;
mod profiling;
mod encode;
mod send;

mod utils;

use std::cmp::max;
use std::env;
use std::thread::{self};
use std::time::{Duration, Instant};

use log::{debug, error, info};
use profiling::FrameStats;

use std::net::{SocketAddr, TcpListener, UdpSocket};

use scrap::{Capturer, Display, Frame};
use send::udp::UDPFrameSender;

use crate::encode::Encoder;

use crate::encode::ffmpeg::h264::H264Encoder;
use crate::encode::ffmpeg::h264rgb::H264RGBEncoder;
use crate::encode::ffmpeg::h265::H265Encoder;
use crate::encode::identity::IdentityEncoder;
use crate::encode::yuv420p::YUV420PEncoder;
use crate::profiling::RoundStats;
use crate::send::tcp::TCPFrameSender;
use crate::send::FrameSender;
use crate::utils::encoding::setup_encoding_env;

#[allow(dead_code)]
fn enstablish_udp_connection() -> std::io::Result<(UdpSocket, SocketAddr)> {
    let socket = UdpSocket::bind("127.0.0.1:5001")?;

    info!("Socket bound, waiting for hello message...");

    let mut hello_buffer = [0; 16];
    let (bytes_received, client_address) = socket.recv_from(&mut hello_buffer)?;
    assert_eq!(bytes_received, 16);
    // let client_address = SocketAddr::from_str("127.0.0.1:5000").unwrap();

    info!("Hello message received correctly. Streaming...");
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();

    Ok((socket, client_address))
}

fn main() -> std::io::Result<()> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");

    const FPS: i64 = 60;
    let spin_time = 1000 / FPS;

    /*const PACKET_SIZE: usize = 512;
    let (udp_socket, client_address) = enstablish_udp_connection()?;
    let mut frame_sender = UDPFrameSender::new(&udp_socket, PACKET_SIZE, &client_address);*/

    let (mut packed_bgr_frame_buffer, mut encoder) = setup_encoding_env(&capturer, &args[1]);

    let listener = TcpListener::bind("127.0.0.1:5001")?;
    info!("Waiting for client connection...");
    let (mut stream, _client_address) = listener.accept()?;
    let mut frame_sender = TCPFrameSender::new(&mut stream);

    let round_duration = Duration::from_secs(1);
    let mut last_frame_transmission_time = 0;

    let mut round_stats: RoundStats = RoundStats::default();

    loop {
        thread::sleep(Duration::from_millis(
            max(0, spin_time - last_frame_transmission_time) as u64,
        ));

        match transmit_frame(
            &mut capturer,
            &mut packed_bgr_frame_buffer,
            &mut *encoder,
            &mut frame_sender,
        ) {
            Ok(frame_stats) => {
                last_frame_transmission_time = frame_stats.total_time as i64;
                round_stats.profile_frame(frame_stats);

                let current_round_duration = round_stats.start_time.elapsed();

                if current_round_duration.gt(&round_duration) {
                    round_stats.print_round_stats();
                    round_stats.reset();
                }
            }
            Err(e) => error!("Frame transmission error: {}", e),
        };
    }
}
fn transmit_frame(
    capturer: &mut Capturer,
    packed_bgr_frame_buffer: &mut [u8],
    encoder: &mut dyn Encoder,
    frame_sender: &mut dyn FrameSender,
) -> Result<FrameStats, std::io::Error> {
    let loop_start_time = Instant::now();

    // Capture frame
    let result = capture::capture_frame(capturer);

    debug!("Frame captured");

    let packed_bgra_frame_buffer: Frame = match result {
        Ok(buffer) => buffer,
        Err(error) => {
            return Err(error);
        }
    };

    debug!("Encoding...");

    let encoding_start_time = Instant::now();

    debug!(
        "{} {}",
        packed_bgra_frame_buffer.len(),
        packed_bgr_frame_buffer.len()
    );

    packed_bgra_to_packed_bgr(&packed_bgra_frame_buffer, packed_bgr_frame_buffer);
    let encoded_size = encoder.encode(&packed_bgr_frame_buffer);

    let encoding_time = encoding_start_time.elapsed().as_millis();

    debug!("Encoding time: {}", encoding_time);

    debug!(
        "Encoded frame size: {}/{}",
        encoded_size,
        packed_bgra_frame_buffer.len()
    );

    let transfer_start_time = Instant::now();

    debug!(
        "Encoded frame slice length: {}",
        encoder.get_encoded_frame().len()
    );

    frame_sender.send_frame(encoder.get_encoded_frame());

    let transfer_time = transfer_start_time.elapsed().as_millis();
    debug!("Transfer time: {}", transfer_time);

    let total_time = loop_start_time.elapsed().as_millis();
    debug!("Total time: {}", total_time);

    Ok(FrameStats {
        encoding_time,
        transfer_time,
        total_time,
        encoded_size,
    })
}

fn packed_bgra_to_packed_bgr(packed_bgra_buffer: &[u8], packed_bgr_buffer: &mut [u8]) {
    let pixels_count = packed_bgra_buffer.len() / 4;

    for i in 0..pixels_count {
        packed_bgr_buffer[i * 3] = packed_bgra_buffer[i * 4];
        packed_bgr_buffer[i * 3 + 1] = packed_bgra_buffer[i * 4 + 1];
        packed_bgr_buffer[i * 3 + 2] = packed_bgra_buffer[i * 4 + 2];
    }
}
