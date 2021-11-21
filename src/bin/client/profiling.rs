use std::time::Instant;

use log::info;

pub struct ReceivedFrameStats {
    pub reception_time: u128,
    pub decoding_time: u128,
    pub rendering_time: u128,
    pub total_time: u128,

    pub rendered: bool,
}

pub struct ReceptionRoundStats {
    pub start_time: Instant,
    pub profiled_frames: Vec<ReceivedFrameStats>,
}

impl Default for ReceptionRoundStats {
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

macro_rules! field_vec {
    ($data_vec:expr, $field_name:ident, $data_type:ty) => {
        $data_vec
            .iter()
            .map(|o| o.$field_name)
            .collect::<Vec<$data_type>>()
    };
}

impl ReceptionRoundStats {
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.profiled_frames.clear();
    }

    pub fn profile_frame(&mut self, frame_stats: ReceivedFrameStats) {
        self.profiled_frames.push(frame_stats);
    }

    pub fn print_round_stats(&mut self) {
        info!("Reception round stats: ");

        info!(
            "Received {} frames in {} seconds",
            self.profiled_frames.len(),
            self.start_time.elapsed().as_secs()
        );

        info!(
            "Dropped frames: {}",
            self.profiled_frames
                .iter()
                .filter(|frame| !frame.rendered)
                .count()
        );

        info!(
            "Average reception time: {}ms",
            vec_avg!(field_vec!(self.profiled_frames, reception_time, u128), u128)
        );

        info!(
            "Average decoding time: {}ms",
            vec_avg!(field_vec!(self.profiled_frames, decoding_time, u128), u128)
        );

        info!(
            "Average rendering time: {}ms",
            vec_avg!(field_vec!(self.profiled_frames, rendering_time, u128), u128)
        );

        info!(
            "Average total time: {}ms",
            vec_avg!(field_vec!(self.profiled_frames, total_time, u128), u128)
        );
    }
}
