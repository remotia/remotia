extern crate scrap;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use object_pool::Pool;

use std::cmp::max;
use std::thread::{self};
use std::time::Duration;

use crate::server::capture::{self, FrameCapturer};
use crate::server::encode::Encoder;
use crate::server::profiling::TransmittedFrameStats;
use crate::server::send::FrameSender;

use crate::server::utils::encoding::{packed_bgra_to_packed_bgr, setup_packed_bgr_frame_buffer};
use crate::server::utils::profilation::setup_round_stats;
use clap::Parser;
use log::{debug, error, info};

use scrap::{Capturer, Display, Frame};

pub struct SiloServerConfiguration {
    pub frame_capturer: Arc<Mutex<Box<dyn FrameCapturer + Send>>>,
    pub encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    pub frame_sender: Arc<Mutex<Box<dyn FrameSender + Send>>>,

    pub console_profiling: bool,
    pub csv_profiling: bool,

    pub width: usize,
    pub height: usize,
}

pub struct SiloServerPipeline {
    config: SiloServerConfiguration,
}

impl SiloServerPipeline {
    pub fn new(config: SiloServerConfiguration) -> SiloServerPipeline {
        Self { config }
    }

    pub async fn run(mut self) {
        const FPS: i64 = 60;
        let spin_time = 1000 / FPS;

        let packed_bgr_frame_buffers_pool = Pool::new(3, || {
            setup_packed_bgr_frame_buffer(self.config.width, self.config.height)
        });

        let encoded_frame_buffers_pool = Pool::new(3, || {
            Vec::with_capacity(self.config.width * self.config.height * 3)
        });

        let round_duration = Duration::from_secs(1);
        let mut last_frame_transmission_time = 0;

        let mut round_stats =
            setup_round_stats(self.config.csv_profiling, self.config.console_profiling).unwrap();

        let mut transfer_handle;

        loop {
            thread::sleep(Duration::from_millis(
                max(0, spin_time - last_frame_transmission_time) as u64,
            ));

            let mut frame_stats = TransmittedFrameStats::default();

            let frame_capturer = self.config.frame_capturer.clone();

            let mut owned_frame_capturer = frame_capturer.lock().unwrap();
            // Capture frame
            let capture_start_time = Instant::now();
            let result = owned_frame_capturer.capture();
            debug!("Frame captured");

            let packed_bgra_frame_buffer = result.unwrap();

            frame_stats.capture_time = capture_start_time.elapsed().as_millis();

            let encoder = self.config.encoder.clone();

            debug!("Encoding...");

            let mut owned_encoder = encoder.lock().unwrap();

            let encoding_start_time = Instant::now();

            let mut packed_bgr_frame_buffer = packed_bgr_frame_buffers_pool.try_pull().unwrap();

            packed_bgra_to_packed_bgr(&packed_bgra_frame_buffer, &mut packed_bgr_frame_buffer);

            let mut encoded_frame_buffer = encoded_frame_buffers_pool.try_pull().unwrap();
            frame_stats.encoded_size = owned_encoder.encode(&packed_bgr_frame_buffer, &mut encoded_frame_buffer);
            frame_stats.encoding_time = encoding_start_time.elapsed().as_millis();

            let frame_sender = self.config.frame_sender.clone();

            /*let transfer_handle = tokio::spawn(async move {
                debug!("Transferring...");

                let transfer_start_time = Instant::now();

                let owned_frame_sender = frame_sender.lock().unwrap();

                owned_frame_sender
                    .send_frame(encoded_frame_buffer.as_slice())
                    .await;

                frame_stats.transfer_time = transfer_start_time.elapsed().as_millis();
            });*/

            transfer_handle.await;

            transfer_handle = tokio::spawn(async move {
                frame_sender.lock().unwrap().send_frame(encoded_frame_buffer.as_slice());
            });

            last_frame_transmission_time = frame_stats.total_time as i64;
            round_stats.profile_frame(frame_stats);

            let current_round_duration = round_stats.start_time.elapsed();

            if current_round_duration.gt(&round_duration) {
                round_stats.log();
                round_stats.reset();
            }
        }
    }
}
