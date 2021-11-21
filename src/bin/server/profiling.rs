use std::time::{Duration, Instant};

use log::info;

pub struct TransmittedFrameStats {
    pub encoding_time: u128,
    pub transfer_time: u128,
    pub total_time: u128,

    pub encoded_size: usize,
}

pub struct TransmissionRoundStats {
    pub start_time: Instant,

    pub transmitted_frames: u16,

    pub encoding_times: Vec<u128>,
    pub transfer_times: Vec<u128>,
    pub total_times: Vec<u128>,

    pub encoded_sizes: Vec<usize>,
}

impl Default for TransmissionRoundStats {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            transmitted_frames: 0,
            encoding_times: Vec::new(),
            transfer_times: Vec::new(),
            total_times: Vec::new(),
            encoded_sizes: Vec::new(),
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

        self.transmitted_frames = 0;
        self.encoding_times = Vec::new();
        self.transfer_times = Vec::new();
        self.total_times = Vec::new();
        self.encoded_sizes = Vec::new();
    }

    pub fn profile_frame(&mut self, frame_stats: TransmittedFrameStats) {
        self.transmitted_frames += 1;
        self.encoding_times.push(frame_stats.encoding_time);
        self.transfer_times.push(frame_stats.transfer_time);
        self.total_times.push(frame_stats.total_time);
        self.encoded_sizes.push(frame_stats.encoded_size)
    }

    pub fn print_round_stats(&mut self) {
        info!("Transmission round stats: ");
        info!(
            "Transmitted {} frames in {} seconds",
            self.transmitted_frames,
            self.start_time.elapsed().as_secs()
        );

        info!(
            "Average encoding time: {}ms",
            vec_avg!(self.encoding_times, u128)
        );
        info!(
            "Average transfer time: {}ms",
            vec_avg!(self.transfer_times, u128)
        );
        info!("Average total time: {}ms", vec_avg!(self.total_times, u128));
        info!(
            "Average encoded size: {} bytes",
            vec_avg!(self.encoded_sizes, usize)
        );

        let bandwidth = (self.encoded_sizes.iter().sum::<usize>() as f64) / 1024.0;
        info!(
            "Required round bandwidth: {} Kb ({} Mbits)",
            bandwidth,
            (bandwidth / 1024.0) * 8.0
        );
    }
}
