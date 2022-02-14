use std::time::{Duration, Instant};

use crate::{
    common::feedback::FeedbackMessage,
    server::profiling::ServerProfiler,
    error::DropReason, 
    types::FrameData,
};

use async_trait::async_trait;
use log::info;

pub struct ConsoleDropReasonsProfiler {
    pub types_to_log: Vec<DropReason>,
    pub round_duration: Duration,
    pub current_round_start: Instant,
    pub logged_frames: Vec<FrameData>,
}

impl Default for ConsoleDropReasonsProfiler {
    fn default() -> Self {
        Self {
            types_to_log: Vec::new(),
            round_duration: Duration::from_secs(1),
            current_round_start: Instant::now(),
            logged_frames: Vec::new(),
        }
    }
}

impl ConsoleDropReasonsProfiler {
    fn print_round_stats(&self) {
        info!("Errors");

        let logged_frames_count = self.logged_frames.len() as u128;

        if logged_frames_count == 0 {
            info!("No successfully transmitted frames");
            return;
        } else {
            info!("Logged frames: {}", logged_frames_count);
        }

        self.types_to_log.iter().for_each(|error_type| {
            let error_type = *error_type;
            let count = self
                .logged_frames
                .iter()
                .filter(|frame| frame.get_drop_reason().is_some())
                .filter(|frame| {
                    std::mem::discriminant(&frame.get_drop_reason().unwrap())
                        == std::mem::discriminant(&error_type)
                })
                .count();

            info!("{}: {}", error_type, count);
        });
    }

    fn reset_round(&mut self) {
        self.logged_frames.clear();
        self.current_round_start = Instant::now();
    }
}

#[async_trait]
impl ServerProfiler for ConsoleDropReasonsProfiler {
    fn log_frame(&mut self, frame_data: FrameData) {
        self.logged_frames.push(frame_data);

        if self.current_round_start.elapsed().gt(&self.round_duration) {
            self.print_round_stats();
            self.reset_round();
        }
    }

    async fn pull_feedback(&mut self) -> Option<FeedbackMessage> {
        None
    }
}
