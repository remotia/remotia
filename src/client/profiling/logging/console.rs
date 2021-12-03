use log::info;

use crate::{
    client::{
        error::ClientError,
        profiling::{ReceivedFrameStats, ReceptionRoundStats},
    },
    field_vec, vec_avg,
};

use super::ReceptionRoundLogger;

macro_rules! is_error_of_type {
    ($frame_error: expr, $expected_error: path) => {
        if let Some(error) = $frame_error {
            let result = match error {
                $expected_error => true,
                _ => false,
            };

            result
        } else {
            false
        }
    };
}

#[derive(Default)]
pub struct ReceptionRoundConsoleLogger {}

impl ReceptionRoundLogger for ReceptionRoundConsoleLogger {
    fn log(&mut self, profiled_frames: &Vec<ReceivedFrameStats>) {
        info!("Reception round stats: ");

        info!("Received {} frames", profiled_frames.len(),);

        let dropped_frames = profiled_frames
            .iter()
            .filter(|frame| frame.error.is_some())
            .count();

        info!("Total dropped frames: {}, of which:", dropped_frames);

        let timed_out_frames = profiled_frames
            .iter()
            .filter(|frame| is_error_of_type!(&frame.error, ClientError::Timeout))
            .count();

        info!("Timeouts: {}", timed_out_frames);

        info!(
            "Average reception time: {}ms",
            vec_avg!(field_vec!(profiled_frames, reception_time, u128), u128)
        );

        info!(
            "Average decoding time: {}ms",
            vec_avg!(field_vec!(profiled_frames, decoding_time, u128), u128)
        );

        info!(
            "Average rendering time: {}ms",
            vec_avg!(field_vec!(profiled_frames, rendering_time, u128), u128)
        );

        info!(
            "Average total time: {}ms",
            vec_avg!(field_vec!(profiled_frames, total_time, u128), u128)
        );
    }
}
