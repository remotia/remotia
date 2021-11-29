use std::time::Instant;

pub mod logging;

use log::info;
use serde::Serialize;

use self::logging::{ReceptionRoundLogger, console::ReceptionRoundConsoleLogger};

#[derive(Serialize)]
pub struct ReceivedFrameStats {
    pub reception_time: u128,
    pub decoding_time: u128,
    pub rendering_time: u128,
    pub total_time: u128,

    pub rendered: bool,
}

pub struct ReceptionRoundStats {
    pub start_time: Instant,
    pub profiled_frames: Vec<ReceivedFrameStats>,

    pub loggers: Vec<Box<dyn ReceptionRoundLogger>>
}

impl Default for ReceptionRoundStats {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            profiled_frames: Vec::new(),
            loggers: vec![Box::new(ReceptionRoundConsoleLogger { })]
        }
    }
}

impl ReceptionRoundStats {
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.profiled_frames.clear();
    }

    pub fn profile_frame(&mut self, frame_stats: ReceivedFrameStats) {
        self.profiled_frames.push(frame_stats);
    }

    pub fn log(&mut self) {
        for i in 0..self.loggers.len() {
            self.loggers[i].log(&self.profiled_frames);
        }
    }
}