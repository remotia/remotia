use log::info;

use crate::{client::profiling::{ReceptionRoundStats, ReceivedFrameStats}, field_vec, vec_avg};

use super::ReceptionRoundLogger;

#[derive(Default)]
pub struct ReceptionRoundConsoleLogger { }

impl ReceptionRoundLogger for ReceptionRoundConsoleLogger {
    fn log(&mut self, profiled_frames: &Vec<ReceivedFrameStats>) {
        info!("Reception round stats: ");

        info!(
            "Received {} frames",
            profiled_frames.len(),
        );

        info!(
            "Dropped frames: {}",
            profiled_frames
                .iter()
                .filter(|frame| !frame.rendered)
                .count()
        );

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