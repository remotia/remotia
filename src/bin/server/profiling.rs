use std::time::{Duration, Instant};

use log::info;
use remotia::field_vec;

pub struct TransmittedFrameStats {
    pub encoding_time: u128,
    pub transfer_time: u128,
    pub total_time: u128,

    pub encoded_size: usize,
}

pub struct TransmissionRoundStats {
    pub start_time: Instant,
    pub profiled_frames: Vec<TransmittedFrameStats>,
}

impl Default for TransmissionRoundStats {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            profiled_frames: Vec::new(),
        }
    }
}

macro_rules! vec_avg {
    ($data_vec:expr, $data_type:ty) => {
        $data_vec.iter().sum::<$data_type>() / $data_vec.len() as $data_type
    };
}

impl TransmissionRoundStats {
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.profiled_frames = Vec::new();
    }

    pub fn profile_frame(&mut self, frame_stats: TransmittedFrameStats) {
        self.profiled_frames.push(frame_stats);
    }

    pub fn print_round_stats(&mut self) {
        info!("Transmission round stats: ");
        info!(
            "Transmitted {} frames in {} seconds",
            self.profiled_frames.len(),
            self.start_time.elapsed().as_secs()
        );

        info!(
            "Average encoding time: {}ms",
            vec_avg!(field_vec!(self.profiled_frames, encoding_time, u128), u128)
        );

        info!(
            "Average transfer time: {}ms",
            vec_avg!(field_vec!(self.profiled_frames, transfer_time, u128), u128)
        );
        info!(
            "Average total time: {}ms",
            vec_avg!(field_vec!(self.profiled_frames, total_time, u128), u128)
        );
        info!(
            "Average encoded size: {} bytes",
            vec_avg!(field_vec!(self.profiled_frames, encoded_size, usize), usize)
        );

        let bandwidth = (field_vec!(self.profiled_frames, encoded_size, usize)
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
