use std::time::{Duration, Instant};

use log::info;
use remotia::{field_vec, vec_avg};
use serde::Serialize;

use self::logging::{TransmissionRoundLogger, console::TransmissionRoundConsoleLogger};

pub mod logging;

#[derive(Serialize)]
pub struct TransmittedFrameStats {
    pub encoding_time: u128,
    pub transfer_time: u128,
    pub total_time: u128,

    pub encoded_size: usize,
}

pub struct TransmissionRoundStats {
    pub(super) start_time: Instant,
    pub(super) profiled_frames: Vec<TransmittedFrameStats>,

    pub loggers: Vec<Box<dyn TransmissionRoundLogger>>
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

