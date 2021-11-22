use log::info;
use remotia::{field_vec, vec_avg};

use crate::profiling::{TransmissionRoundStats, TransmittedFrameStats};

use super::TransmissionRoundLogger;

pub struct TransmissionRoundConsoleLogger { }

impl TransmissionRoundLogger for TransmissionRoundConsoleLogger {
    fn log(&mut self, profiled_frames: &Vec<TransmittedFrameStats>) {
        info!("Transmission round stats: ");
        info!(
            "Transmitted {} frames",
            profiled_frames.len()
        );

        info!(
            "Average encoding time: {}ms",
            vec_avg!(field_vec!(profiled_frames, encoding_time, u128), u128)
        );

        info!(
            "Average transfer time: {}ms",
            vec_avg!(field_vec!(profiled_frames, transfer_time, u128), u128)
        );
        info!(
            "Average total time: {}ms",
            vec_avg!(field_vec!(profiled_frames, total_time, u128), u128)
        );
        info!(
            "Average encoded size: {} bytes",
            vec_avg!(field_vec!(profiled_frames, encoded_size, usize), usize)
        );

        let bandwidth = (field_vec!(profiled_frames, encoded_size, usize)
            .iter()
            .sum::<usize>() as f64)
            / 1024.0;

        info!(
            "Required round bandwidth: {} Kb ({} Mbits)",
            bandwidth,
            (bandwidth / 1024.0) * 8.0
        );
    }
}
