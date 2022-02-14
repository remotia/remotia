use std::time::{Duration, Instant};

use remotia::{error::DropReason, traits::FrameProcessor, types::FrameData};

use async_trait::async_trait;
use log::info;

pub struct ConsoleDropReasonLogger {
    header: Option<String>,
    types_to_log: Vec<DropReason>,
    round_duration: Duration,

    current_round_start: Instant,

    logged_reasons: Vec<DropReason>,
}

impl Default for ConsoleDropReasonLogger {
    fn default() -> Self {
        Self {
            header: None,
            types_to_log: Vec::new(),
            round_duration: Duration::from_secs(1),
            current_round_start: Instant::now(),
            logged_reasons: Vec::new(),
        }
    }
}

impl ConsoleDropReasonLogger {
    pub fn new() -> Self {
        Self::default()
    }

    // Building functions
    pub fn header(mut self, header: &str) -> Self {
        self.header = Some(header.to_string());
        self
    }

    pub fn log(mut self, value: DropReason) -> Self {
        self.types_to_log.push(value);
        self
    }

    // Logging functions
    fn print_round_stats(&self) {
        if self.header.is_some() {
            info!("{}", self.header.as_ref().unwrap());
        }

        let dropped_frames_count = self.logged_reasons.len() as u128;

        if dropped_frames_count == 0 {
            info!("No successfully transmitted frames");
            return;
        } else {
            info!("Dropped frames: {}", dropped_frames_count);
        }

        self.types_to_log.iter().for_each(|reason_type| {
            let reason_type = *reason_type;
            let count = self
                .logged_reasons
                .iter()
                .filter(|reason| {
                    std::mem::discriminant(*reason) == std::mem::discriminant(&reason_type)
                })
                .count();

            if count > 0 {
                info!("{}: {}", reason_type, count);
            }
        });
    }

    fn reset_round(&mut self) {
        self.logged_reasons.clear();
        self.current_round_start = Instant::now();
    }

    fn log_frame_data(&mut self, frame_data: &FrameData) {
        if frame_data.get_drop_reason().is_none() {
            return;
        }

        self.logged_reasons
            .push(frame_data.get_drop_reason().clone().unwrap());

        if self.current_round_start.elapsed().gt(&self.round_duration) {
            self.print_round_stats();
            self.reset_round();
        }
    }
}

#[async_trait]
impl FrameProcessor for ConsoleDropReasonLogger {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        self.log_frame_data(&frame_data);
        Some(frame_data)
    }
}
