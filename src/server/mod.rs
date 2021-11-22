pub mod capture;
pub mod encode;
pub mod profiling;
pub mod send;

pub mod utils;

extern crate scrap;

use std::cmp::max;
use std::thread::{self};
use std::time::Duration;

use crate::server::send::tcp::TCPFrameSender;
use crate::server::utils::encoding::{setup_packed_bgr_frame_buffer};
use crate::server::utils::profilation::setup_round_stats;
use crate::server::utils::transmission::transmit_frame;
use clap::Parser;
use log::{error, info};

use std::net::TcpListener;

use scrap::{Capturer, Display};

use self::encode::Encoder;
use self::send::FrameSender;

pub struct ServerConfiguration {
    pub encoder: Box<dyn Encoder>,
    pub frame_sender: Box<dyn FrameSender>
}

pub fn run_with_configuration(
    mut config: ServerConfiguration,
) -> std::io::Result<()> {
    let display = Display::primary().expect("Couldn't find primary display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");

    const FPS: i64 = 60;
    let spin_time = 1000 / FPS;

    let mut packed_bgr_frame_buffer =
        setup_packed_bgr_frame_buffer(capturer.width(), capturer.height());

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
            &mut *config.encoder,
            &mut *config.frame_sender,
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
