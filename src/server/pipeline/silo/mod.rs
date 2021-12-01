extern crate scrap;

mod capture;
mod encode;
mod profile;
mod transfer;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use tokio::sync::mpsc::{self, Receiver, Sender};

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
        const FPS: i64 = 60;
        let spin_time = 1000 / FPS;

        /*let raw_frame_buffers_pool = Arc::new(Pool::new(1, || {
            let mut buf = BytesMut::with_capacity(frame_size);
            buf.resize(frame_size, 0);
            buf
        }));

        let encoded_frame_buffers_pool = Arc::new(Pool::new(1, || {
            let mut buf = BytesMut::with_capacity(frame_size);
            buf.resize(frame_size, 0);
            buf
        }));*/

        const MAXIMUM_RAW_FRAME_BUFFERS: usize = 1;
        const MAXIMUM_ENCODED_FRAME_BUFFERS: usize = 1;

        let frame_size = self.config.width * self.config.height * 3;

        let (raw_frame_buffers_sender,  raw_frame_buffers_receiver,) = mpsc::channel::<BytesMut>(MAXIMUM_RAW_FRAME_BUFFERS);
        let (encoded_frame_buffers_sender,  encoded_frame_buffers_receiver,) = mpsc::channel::<BytesMut>(MAXIMUM_ENCODED_FRAME_BUFFERS);

        for _ in 0..MAXIMUM_RAW_FRAME_BUFFERS {
            let mut buf = BytesMut::with_capacity(frame_size);
            buf.resize(frame_size, 0);
            raw_frame_buffers_sender.send(buf).await.unwrap();
        }

        for _ in 0..MAXIMUM_ENCODED_FRAME_BUFFERS {
            let mut buf = BytesMut::with_capacity(frame_size);
            buf.resize(frame_size, 0);
            encoded_frame_buffers_sender.send(buf).await.unwrap();
        }

        let round_duration = Duration::from_secs(1);
        // let mut last_frame_transmission_time = 0;

        let (capture_result_sender, capture_result_receiver) = mpsc::channel::<CaptureResult>(1);
        let (encode_result_sender, encode_result_receiver) = mpsc::channel::<EncodeResult>(1);
        let (transfer_result_sender, transfer_result_receiver) = mpsc::channel::<TransferResult>(1);

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
