use super::TransmissionRoundStats;

pub mod console;

pub trait TransmissionRoundLogger {
    fn log(&self, round_stats: &TransmissionRoundStats);
}

