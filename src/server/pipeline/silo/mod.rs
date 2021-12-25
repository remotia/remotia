extern crate scrap;

mod capture;
mod encode;
mod profile;
mod transfer;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use async_trait::async_trait;
use bytes::BytesMut;
use tokio::task::JoinHandle;

use std::cmp::max;
use std::thread::{self};
use std::time::Duration;

use crate::server::capture::FrameCapturer;
use crate::server::encode::Encoder;
use crate::server::pipeline::silo::capture::{launch_capture_thread, CaptureResult};
use crate::server::pipeline::silo::encode::{launch_encode_thread, EncodeResult};
use crate::server::pipeline::silo::profile::launch_profile_thread;
use crate::server::pipeline::silo::transfer::{launch_transfer_thread, TransferResult};
use crate::server::profiling::TransmittedFrameStats;
use crate::server::send::FrameSender;

use crate::server::utils::encoding::{packed_bgra_to_packed_bgr, setup_packed_bgr_frame_buffer};
use crate::server::utils::profilation::setup_round_stats;
use clap::Parser;
use log::{debug, error, info};

use scrap::{Capturer, Display, Frame};

pub struct SiloServerConfiguration {
    pub frame_capturer: Box<dyn FrameCapturer + Send>,
    pub encoder: Box<dyn Encoder + Send>,
    pub frame_sender: Box<dyn FrameSender + Send>,

    pub console_profiling: bool,
    pub csv_profiling: bool,

    pub target_fps: u32,

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

    pub async fn run(self) {
        let spin_time = (1000 / self.config.target_fps) as i64;

        const MAXIMUM_CAPTURE_DELAY: u128 = 30000;
        const MAXIMUM_RAW_FRAME_BUFFERS: usize = 1;
        const MAXIMUM_ENCODED_FRAME_BUFFERS: usize = 16;

        let raw_frame_size = self.config.width * self.config.height * 3;
        let maximum_encoded_frame_size = self.config.width * self.config.height * 3;

        let (raw_frame_buffers_sender,  raw_frame_buffers_receiver,) = mpsc::unbounded_channel::<BytesMut>();
        let (encoded_frame_buffers_sender,  encoded_frame_buffers_receiver,) = mpsc::unbounded_channel::<BytesMut>();

        for _ in 0..MAXIMUM_RAW_FRAME_BUFFERS {
            let mut buf = BytesMut::with_capacity(raw_frame_size);
            buf.resize(raw_frame_size, 0);
            raw_frame_buffers_sender.send(buf).unwrap();
        }

        for _ in 0..MAXIMUM_ENCODED_FRAME_BUFFERS {
            let mut buf = BytesMut::with_capacity(maximum_encoded_frame_size);
            buf.resize(maximum_encoded_frame_size, 0);
            encoded_frame_buffers_sender.send(buf).unwrap();
        }

        let round_duration = Duration::from_secs(1);

        let (capture_result_sender, capture_result_receiver) = mpsc::unbounded_channel::<CaptureResult>();
        let (encode_result_sender, encode_result_receiver) = mpsc::unbounded_channel::<EncodeResult>();
        let (transfer_result_sender, transfer_result_receiver) = mpsc::unbounded_channel::<TransferResult>();

        let capture_handle = launch_capture_thread(
            spin_time,
            raw_frame_buffers_receiver,
            capture_result_sender,
            self.config.frame_capturer,
        );

        let encode_handle = launch_encode_thread(
            self.config.encoder,
            raw_frame_buffers_sender,
            encoded_frame_buffers_receiver,
            capture_result_receiver,
            encode_result_sender,
            MAXIMUM_CAPTURE_DELAY
        );

        let transfer_handle = launch_transfer_thread(
            self.config.frame_sender,
            encoded_frame_buffers_sender,
            encode_result_receiver,
            transfer_result_sender,
        );

        let profile_handle = launch_profile_thread(
            self.config.csv_profiling,
            self.config.console_profiling,
            transfer_result_receiver,
            round_duration,
        );

        capture_handle.await.unwrap();
        encode_handle.await.unwrap();
        transfer_handle.await.unwrap();
        profile_handle.await.unwrap();
    }
}
