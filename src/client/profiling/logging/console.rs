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

        info!("Profiled {} frame receptions", profiled_frames.len());

        let rendered_frames: Vec<&ReceivedFrameStats> = profiled_frames
            .iter()
            .filter(|frame| frame.error.is_none())
            .collect();

        let dropped_frames_count = profiled_frames.len() - rendered_frames.len();

        info!("Total rendered frames: {}", rendered_frames.len());

        info!("Total dropped frames: {}, of which:", dropped_frames_count);

        info!("Timeouts: {}", profiled_frames
            .iter()
            .filter(|frame| is_error_of_type!(&frame.error, ClientError::Timeout))
            .count());

        info!("No complete frames: {}", profiled_frames
            .iter()
            .filter(|frame| is_error_of_type!(&frame.error, ClientError::NoCompleteFrames))
            .count());

        info!("Stale frames: {}", profiled_frames
            .iter()
            .filter(|frame| is_error_of_type!(&frame.error, ClientError::StaleFrame))
            .count());



        if rendered_frames.len() == 0 {
            return;
        }

        info!(
            "Average reception time: {}ms",
            vec_avg!(field_vec!(rendered_frames, reception_time, u128), u128)
        );

        info!(
            "Average decoding time: {}ms",
            vec_avg!(field_vec!(rendered_frames, decoding_time, u128), u128)
        );

        info!(
            "Average rendering time: {}ms",
            vec_avg!(field_vec!(rendered_frames, rendering_time, u128), u128)
        );

        info!(
            "Average total time: {}ms",
            vec_avg!(field_vec!(rendered_frames, total_time, u128), u128)
        );

        info!(
            "Average receiver idle time: {}ms",
            vec_avg!(field_vec!(rendered_frames, receiver_idle_time, u128), u128)
        );

        info!(
            "Average decoder idle time: {}ms",
            vec_avg!(field_vec!(rendered_frames, decoder_idle_time, u128), u128)
        );

        info!(
            "Average renderer idle time: {}ms",
            vec_avg!(field_vec!(rendered_frames, renderer_idle_time, u128), u128)
        );

        info!(
            "Average frame delay: {}ms",
            vec_avg!(field_vec!(rendered_frames, frame_delay, u128), u128)
        );

        info!(
            "Average spin time: {}ms",
            vec_avg!(field_vec!(rendered_frames, spin_time, u64), u64)
        );

        info!(
            "Average reception delay: {}ms",
            vec_avg!(field_vec!(rendered_frames, reception_delay, u128), u128)
        );
    }
}
