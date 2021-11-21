use log::info;
use remotia::{field_vec, vec_avg};

use crate::profiling::TransmissionRoundStats;

use super::TransmissionRoundLogger;

pub struct TransmissionRoundConsoleLogger { }

impl TransmissionRoundLogger for TransmissionRoundConsoleLogger {
    fn log(&self, round_stats: &TransmissionRoundStats) {
        info!("Transmission round stats: ");
        info!(
            "Transmitted {} frames in {} seconds",
            round_stats.profiled_frames.len(),
            round_stats.start_time.elapsed().as_secs()
        );

        info!(
            "Average encoding time: {}ms",
            vec_avg!(field_vec!(round_stats.profiled_frames, encoding_time, u128), u128)
        );

        info!(
            "Average transfer time: {}ms",
            vec_avg!(field_vec!(round_stats.profiled_frames, transfer_time, u128), u128)
        );
        info!(
            "Average total time: {}ms",
            vec_avg!(field_vec!(round_stats.profiled_frames, total_time, u128), u128)
        );
        info!(
            "Average encoded size: {} bytes",
            vec_avg!(field_vec!(round_stats.profiled_frames, encoded_size, usize), usize)
        );

        let bandwidth = (field_vec!(round_stats.profiled_frames, encoded_size, usize)
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
