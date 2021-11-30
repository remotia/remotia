extern crate scrap;

use std::time::Instant;

use async_trait::async_trait;

use std::cmp::max;
use std::thread::{self};
use std::time::Duration;

use crate::server::capture;
use crate::server::encode::Encoder;
use crate::server::profiling::TransmittedFrameStats;
use crate::server::send::FrameSender;

use crate::server::utils::encoding::{packed_bgra_to_packed_bgr, setup_packed_bgr_frame_buffer};
use crate::server::utils::profilation::setup_round_stats;
use clap::Parser;
use log::{error, info, debug};

use scrap::{Capturer, Display, Frame};

pub struct WaterfallServerConfiguration {
    pub encoder: Box<dyn Encoder>,
    pub frame_sender: Box<dyn FrameSender>,

    pub console_profiling: bool,
    pub csv_profiling: bool,
}

pub struct WaterfallPipeline {
    config: WaterfallServerConfiguration,
}

impl WaterfallPipeline {
    pub fn new(config: WaterfallServerConfiguration) -> Self {
        Self { config }
    }

    pub async fn run(mut self) {
        let display = Display::primary().expect("Couldn't find primary display.");
        let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");

        const FPS: i64 = 60;
        let spin_time = 1000 / FPS;

        let mut packed_bgr_frame_buffer =
            setup_packed_bgr_frame_buffer(capturer.width(), capturer.height());

        let round_duration = Duration::from_secs(1);
        let mut last_frame_transmission_time = 0;

        let mut round_stats = setup_round_stats(self.config.csv_profiling, self.config.console_profiling).unwrap();

        loop {
            thread::sleep(Duration::from_millis(
                max(0, spin_time - last_frame_transmission_time) as u64,
            ));

            match self.transmit_frame(
                &mut capturer,
                &mut packed_bgr_frame_buffer,
            )
            .await
            {
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

    async fn transmit_frame(
        &mut self, 
        capturer: &mut Capturer,
        packed_bgr_frame_buffer: &mut [u8],
    ) -> Result<TransmittedFrameStats, std::io::Error> {
        let loop_start_time = Instant::now();

        // Capture frame
        let capture_start_time = Instant::now();
        let result = capture::capture_frame(capturer);
        let capture_time = capture_start_time.elapsed().as_millis();

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
        let encoded_size = self.config.encoder.encode(&packed_bgr_frame_buffer);

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
            self.config.encoder.get_encoded_frame().len()
        );

        self.config.frame_sender.send_frame(self.config.encoder.get_encoded_frame()).await;

        let transfer_time = transfer_start_time.elapsed().as_millis();
        debug!("Transfer time: {}", transfer_time);

        let total_time = loop_start_time.elapsed().as_millis();
        debug!("Total time: {}", total_time);

        Ok(TransmittedFrameStats {
            capture_time,
            encoding_time,
            transfer_time,
            total_time,
            encoded_size,
        })
    }
}
