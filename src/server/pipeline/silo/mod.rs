extern crate scrap;

mod capture;
mod encode;
mod profile;
mod transfer;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::broadcast;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use async_trait::async_trait;
use bytes::BytesMut;
use tokio::task::JoinHandle;

use std::cmp::max;
use std::thread::{self};
use std::time::Duration;

use crate::common::feedback::FeedbackMessage;
use crate::server::capture::FrameCapturer;
use crate::server::encode::Encoder;
use crate::server::pipeline::silo::capture::{launch_capture_thread, CaptureResult};
use crate::server::pipeline::silo::encode::{launch_encode_thread, EncodeResult};
use crate::server::pipeline::silo::profile::launch_profile_thread;
use crate::server::pipeline::silo::transfer::{launch_transfer_thread, TransferResult};
use crate::server::profiling::TransmittedFrameStats;
use crate::server::send::FrameSender;

use crate::server::profiling::ServerProfiler;
use crate::server::utils::encoding::{packed_bgra_to_packed_bgr, setup_packed_bgr_frame_buffer};
use crate::server::utils::profilation::setup_round_stats;
use clap::Parser;
use log::{debug, error, info};
use scrap::{Capturer, Display, Frame};

pub struct BuffersConfig {
    pub maximum_raw_frame_buffers: usize,
    pub maximum_encoded_frame_buffers: usize
}

pub struct SiloServerConfiguration {
    pub frame_capturer: Box<dyn FrameCapturer + Send>,
    pub encoder: Box<dyn Encoder + Send>,
    pub frame_sender: Box<dyn FrameSender + Send>,

    pub profiler: Box<dyn ServerProfiler + Send>,

    pub console_profiling: bool,
    pub csv_profiling: bool,

    pub frames_capture_rate: u32,

    pub width: usize,
    pub height: usize,

    pub maximum_preencoding_capture_delay: u128,
    pub buffers_conf: BuffersConfig
}

pub struct SiloServerPipeline {
    config: SiloServerConfiguration,
}

impl SiloServerPipeline {
    pub fn new(config: SiloServerConfiguration) -> SiloServerPipeline {
        Self { config }
    }

    pub async fn run(self) {
        let spin_time = (1000 / self.config.frames_capture_rate) as i64;

        let raw_frame_size = self.config.width * self.config.height * 4;
        let maximum_encoded_frame_size = self.config.width * self.config.height * 4;

        let (raw_frame_buffers_sender, raw_frame_buffers_receiver) =
            mpsc::unbounded_channel::<BytesMut>();
        let (encoded_frame_buffers_sender, encoded_frame_buffers_receiver) =
            mpsc::unbounded_channel::<BytesMut>();

        for _ in 0..self.config.buffers_conf.maximum_raw_frame_buffers {
            let mut buf = BytesMut::with_capacity(raw_frame_size);
            buf.resize(raw_frame_size, 0);
            raw_frame_buffers_sender.send(buf).unwrap();
        }

        for _ in 0..self.config.buffers_conf.maximum_encoded_frame_buffers {
            let mut buf = BytesMut::with_capacity(maximum_encoded_frame_size);
            buf.resize(maximum_encoded_frame_size, 0);
            encoded_frame_buffers_sender.send(buf).unwrap();
        }

        let round_duration = Duration::from_secs(1);

        let (capture_result_sender, capture_result_receiver) =
            mpsc::unbounded_channel::<CaptureResult>();
        let (encode_result_sender, encode_result_receiver) =
            mpsc::unbounded_channel::<EncodeResult>();
        let (transfer_result_sender, transfer_result_receiver) =
            mpsc::unbounded_channel::<TransferResult>();

        let (feedback_sender, capturer_feedback_receiver) =
            broadcast::channel::<FeedbackMessage>(32);

        let capture_handle = launch_capture_thread(
            spin_time,
            raw_frame_buffers_receiver,
            capture_result_sender,
            self.config.frame_capturer,
            capturer_feedback_receiver 
        );

        let encode_handle = launch_encode_thread(
            self.config.encoder,
            raw_frame_buffers_sender,
            encoded_frame_buffers_receiver,
            capture_result_receiver,
            encode_result_sender,
            feedback_sender.subscribe(),
            self.config.maximum_preencoding_capture_delay,
        );

        let transfer_handle = launch_transfer_thread(
            self.config.frame_sender,
            encoded_frame_buffers_sender,
            encode_result_receiver,
            transfer_result_sender,
            feedback_sender.subscribe()
        );

        let profile_handle = launch_profile_thread(
            self.config.profiler,
            self.config.csv_profiling,
            self.config.console_profiling,
            transfer_result_receiver,
            feedback_sender,
            round_duration,
        );

        capture_handle.await.unwrap();
        encode_handle.await.unwrap();
        transfer_handle.await.unwrap();
        profile_handle.await.unwrap();
    }
}
