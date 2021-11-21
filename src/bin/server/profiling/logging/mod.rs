use super::TransmissionRoundStats;

pub mod console;
pub mod csv;

pub trait TransmissionRoundLogger {
    fn log(&self, round_stats: &TransmissionRoundStats);
}

