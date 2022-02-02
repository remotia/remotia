use std::time::{Instant};

use serde::Serialize;

use crate::server::profiling::logging::console::TransmissionRoundConsoleLogger;

use self::logging::TransmissionRoundLogger;

use crate::common::feedback::FeedbackMessage;

use async_trait::async_trait;

use super::error::ServerError;

pub mod logging;

pub mod tcp;

#[async_trait]
pub trait ServerProfiler {
    async fn pull_feedback(&mut self) -> Option<FeedbackMessage>;
}

#[derive(Serialize, Default)]
pub struct TransmittedFrameStats {
    pub capture_timestamp: u128,

    pub capture_time: u128,
    pub encoding_time: u128,
    pub transfer_time: u128,
    pub total_time: u128,

    pub capturer_idle_time: u128,
    pub encoder_idle_time: u128,
    pub transferrer_idle_time: u128,

    pub capture_delay: u128,

    pub encoded_size: usize,
    pub transmitted_bytes: usize,

    pub error: Option<ServerError>
}

pub struct TransmissionRoundStats {
    pub start_time: Instant,
    pub profiled_frames: Vec<TransmittedFrameStats>,

    pub loggers: Vec<Box<dyn TransmissionRoundLogger + Send>>
}

impl Default for TransmissionRoundStats {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            profiled_frames: Vec::new(),
            loggers: vec![Box::new(TransmissionRoundConsoleLogger { })]
        }
    }
}

impl TransmissionRoundStats {
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.profiled_frames = Vec::new();
    }

    pub fn profile_frame(&mut self, frame_stats: TransmittedFrameStats) {
        self.profiled_frames.push(frame_stats);
    }

    pub fn log(&mut self) {
        for i in 0..self.loggers.len() {
            self.loggers[i].log(&self.profiled_frames);
        }
    }
}

