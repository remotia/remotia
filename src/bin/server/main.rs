#![allow(unused_imports)]
#![feature(test)]

extern crate scrap;

mod capture;
mod encode;
mod profiling;
mod send;

mod utils;

use std::cmp::max;
use std::env;
use std::thread::{self};
use std::time::{Duration, Instant};

use chrono::Utc;
use clap::Parser;
use log::{debug, error, info};
use profiling::TransmittedFrameStats;

use std::net::{SocketAddr, TcpListener, UdpSocket};

use scrap::{Capturer, Display, Frame};
use send::udp::UDPFrameSender;

use crate::encode::Encoder;

use crate::encode::ffmpeg::h264::H264Encoder;
use crate::encode::ffmpeg::h264rgb::H264RGBEncoder;
use crate::encode::ffmpeg::h265::H265Encoder;
use crate::encode::identity::IdentityEncoder;
use crate::encode::yuv420p::YUV420PEncoder;
use crate::profiling::logging::console::TransmissionRoundConsoleLogger;
use crate::profiling::logging::csv::TransmissionRoundCSVLogger;
use crate::profiling::TransmissionRoundStats;
use crate::send::tcp::TCPFrameSender;
use crate::send::FrameSender;
use crate::utils::encoding::{packed_bgra_to_packed_bgr, setup_encoding_env};

#[derive(Parser)]
#[clap(version = "0.1.0", author = "Lorenzo C. <aegroto@protonmail.com>")]
struct Options {
    #[clap(short, long, default_value = "h264rgb")]
    encoder_name: String
}

fn main() -> std::io::Result<()> {
    env_logger::init();

    let options = Options::parse();

    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");

    const FPS: i64 = 60;
    let spin_time = 1000 / FPS;

    /*const PACKET_SIZE: usize = 512;
    let (udp_socket, client_address) = enstablish_udp_connection()?;
    let mut frame_sender = UDPFrameSender::new(&udp_socket, PACKET_SIZE, &client_address);*/

    let (mut packed_bgr_frame_buffer, mut encoder) = setup_encoding_env(&capturer, &options.encoder_name);

    let listener = TcpListener::bind("127.0.0.1:5001")?;
    info!("Waiting for client connection...");
    let (mut stream, _client_address) = listener.accept()?;
    let mut frame_sender = TCPFrameSender::new(&mut stream);

    let round_duration = Duration::from_secs(1);
    let mut last_frame_transmission_time = 0;

    let mut round_stats = setup_round_stats()?;

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
                    round_stats.log();
                    round_stats.reset();
                }
            }
            Err(e) => error!("Frame transmission error: {}", e),
        };
    }
}

fn setup_round_stats() -> Result<TransmissionRoundStats, std::io::Error> {
    let round_stats: TransmissionRoundStats = {
        let datetime = Utc::now();

        TransmissionRoundStats {
            loggers: vec![
                Box::new(TransmissionRoundCSVLogger::new(
                    format!("csv_logs/server/{}.csv", datetime).as_str(),
                )?),
                Box::new(TransmissionRoundConsoleLogger::default()),
            ],

            ..Default::default()
        }
    };
    Ok(round_stats)
}

fn transmit_frame(
    capturer: &mut Capturer,
    packed_bgr_frame_buffer: &mut [u8],
    encoder: &mut dyn Encoder,
    frame_sender: &mut dyn FrameSender,
) -> Result<TransmittedFrameStats, std::io::Error> {
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

    Ok(TransmittedFrameStats {
        encoding_time,
        transfer_time,
        total_time,
        encoded_size,
    })
}
