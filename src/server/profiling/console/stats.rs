use std::time::{Duration, Instant};

use crate::{
    common::feedback::FeedbackMessage,
    server::{profiling::ServerProfiler, types::ServerFrameData},
};

use async_trait::async_trait;
use log::info;

pub struct ConsoleServerStatsProfiler {
    pub header: Option<String>,
    pub values_to_log: Vec<String>,
    pub round_duration: Duration,

    pub current_round_start: Instant,

    pub logged_frames: Vec<ServerFrameData>,

    pub log_errors: bool,
}

impl Default for ConsoleServerStatsProfiler {
    fn default() -> Self {
        Self {
            header: None,
            values_to_log: Vec::new(),
            round_duration: Duration::from_secs(1),
            current_round_start: Instant::now(),
            logged_frames: Vec::new(),
            log_errors: false,
        }
    }
}

impl ConsoleServerStatsProfiler {
    fn print_round_stats(&self) {
        if self.header.is_some() {
            info!("{}", self.header.as_ref().unwrap());
        }

        let logged_frames_count = self.logged_frames.len() as u128;

        if logged_frames_count == 0 {
            info!("No successfully transmitted frames");
            return;
        } else {
            info!("Logged frames: {}", logged_frames_count);
        }

        self.values_to_log.iter().for_each(|value| {
            let avg = self
                .logged_frames
                .iter()
                .map(|frame| get_frame_stat(frame, value))
                .sum::<u128>()
                / logged_frames_count;

            info!("Average {}: {}", value, avg);
        });
    }

    fn reset_round(&mut self) {
        self.logged_frames.clear();
        self.current_round_start = Instant::now();
    }
}

#[async_trait]
impl ServerProfiler for ConsoleServerStatsProfiler {
    fn log_frame(&mut self, frame_data: ServerFrameData) {
        if !self.log_errors && frame_data.get_error().is_some() {
            return;
        }

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

fn get_frame_stat(frame: &ServerFrameData, key: &str) -> u128 {
    if frame.has(key) {
        frame.get(key)
    } else {
        frame.get_local(key)
    }
}
