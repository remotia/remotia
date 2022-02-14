use std::env;
use std::net::TcpStream;

mod decode;
mod profile;
mod receive;
mod render;

use std::net::SocketAddr;
use std::net::UdpSocket;
use std::ops::ControlFlow;
use std::str::FromStr;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use bytes::BytesMut;
use chrono::Utc;
use clap::Parser;
use log::info;
use log::{debug, error, warn};
use tokio::sync::broadcast;
use tokio::sync::mpsc;

use crate::client::decode::Decoder;
use crate::error::DropReason;
use crate::client::pipeline::silo::decode::launch_decode_thread;
use crate::client::pipeline::silo::decode::DecodeResult;
use crate::client::pipeline::silo::profile::launch_profile_thread;
use crate::client::pipeline::silo::receive::launch_receive_thread;
use crate::client::pipeline::silo::receive::ReceiveResult;
use crate::client::pipeline::silo::render::launch_render_thread;
use crate::client::pipeline::silo::render::RenderResult;
use crate::client::profiling::logging::console::ReceptionRoundConsoleLogger;
use crate::client::profiling::logging::csv::ReceptionRoundCSVLogger;
use crate::client::profiling::ReceivedFrameStats;
use crate::client::profiling::ReceptionRoundStats;
use crate::client::receive::FrameReceiver;
use crate::client::render::Renderer;
use crate::client::utils::profilation::setup_round_stats;
use crate::common::feedback::FeedbackMessage;
use crate::client::profiling::ClientProfiler;

pub struct BuffersConfig {
    pub maximum_encoded_frame_buffers: usize,
    pub maximum_raw_frame_buffers: usize
}

pub struct SiloClientConfiguration {
    pub decoder: Box<dyn Decoder + Send>,
    pub frame_receiver: Box<dyn FrameReceiver + Send>,
    pub renderer: Box<dyn Renderer + Send>,

    pub profiler: Box<dyn ClientProfiler + Send>,

    pub maximum_consecutive_connection_losses: u32,

    pub frames_render_rate: u32,

    pub console_profiling: bool,
    pub csv_profiling: bool,

    pub maximum_pre_render_frame_delay: u128,
    pub buffers_conf: BuffersConfig
}

pub struct SiloClientPipeline {
    config: SiloClientConfiguration,
}

impl SiloClientPipeline {
    pub fn new(config: SiloClientConfiguration) -> Self {
        Self { config }
    }

    pub async fn run(self) {
        info!("Starting to receive stream...");


        let raw_frame_size = self.config.renderer.get_buffer_size();
        let maximum_encoded_frame_size = self.config.renderer.get_buffer_size();

        let (encoded_frame_buffers_sender, encoded_frame_buffers_receiver) =
            mpsc::unbounded_channel::<BytesMut>();
        let (raw_frame_buffers_sender, raw_frame_buffers_receiver) =
            mpsc::unbounded_channel::<BytesMut>();

        for _ in 0..self.config.buffers_conf.maximum_raw_frame_buffers {
            let mut buf = BytesMut::with_capacity(maximum_encoded_frame_size);
            buf.resize(maximum_encoded_frame_size, 0);
            encoded_frame_buffers_sender.send(buf).unwrap();
        }

        for _ in 0..self.config.buffers_conf.maximum_encoded_frame_buffers {
            let mut buf = BytesMut::with_capacity(raw_frame_size);
            buf.resize(raw_frame_size, 0);
            raw_frame_buffers_sender.send(buf).unwrap();
        }

        let (receive_result_sender, receive_result_receiver) =
            mpsc::unbounded_channel::<ReceiveResult>();
        let (decode_result_sender, decode_result_receiver) =
            mpsc::unbounded_channel::<DecodeResult>();
        let (render_result_sender, render_result_receiver) =
            mpsc::unbounded_channel::<RenderResult>();

        let (feedback_sender, receiver_feedback_receiver) =
            broadcast::channel::<FeedbackMessage>(32);

        let receive_handle = launch_receive_thread(
            self.config.frame_receiver,
            encoded_frame_buffers_receiver,
            receive_result_sender,
            receiver_feedback_receiver
        );

        let decode_handle = launch_decode_thread(
            self.config.decoder,
            raw_frame_buffers_receiver,
            encoded_frame_buffers_sender,
            receive_result_receiver,
            decode_result_sender,
            feedback_sender.subscribe()
        );

        let render_handle = launch_render_thread(
            self.config.renderer,
            self.config.frames_render_rate,
            self.config.maximum_pre_render_frame_delay,
            raw_frame_buffers_sender,
            decode_result_receiver,
            render_result_sender,
            feedback_sender.subscribe()
        );

        let profile_handle = launch_profile_thread(
            self.config.profiler,
            render_result_receiver,
            self.config.csv_profiling,
            self.config.console_profiling,
            feedback_sender
        );

        receive_handle.await.unwrap();
        decode_handle.await.unwrap();
        render_handle.await.unwrap();
        profile_handle.await.unwrap();
    }
}
