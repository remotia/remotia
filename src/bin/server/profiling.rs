use std::time::{Duration, Instant};

use log::info;

pub struct RoundStats {
    pub start_time: Instant,

    pub transmitted_frames: u16,

    pub encoding_times: Vec<u128>,
    pub transfer_times: Vec<u128>,
    pub total_times: Vec<u128>,
}

impl Default for RoundStats {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            transmitted_frames: 0,
            encoding_times: Vec::new(),
            transfer_times: Vec::new(),
            total_times: Vec::new(),
        }
    }
}

macro_rules! vec_avg {
    ($data_vec:expr) => {
        $data_vec.iter().sum::<u128>() / $data_vec.len() as u128
    };
}

impl RoundStats {
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.transmitted_frames = 0;
        self.encoding_times = Vec::new();
        self.transfer_times = Vec::new();
        self.total_times = Vec::new();
    }

    pub fn print_round_stats(&mut self) {
        info!("Round stats: ");
        info!(
            "Transmitted {} frames in {} seconds",
            self.transmitted_frames,
            self.start_time.elapsed().as_secs()
        );

        info!("Average encoding time: {}", vec_avg!(self.encoding_times));
        info!("Average transfer time: {}", vec_avg!(self.transfer_times));
        info!("Average total time: {}", vec_avg!(self.total_times));
    }
}
