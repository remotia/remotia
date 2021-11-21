use log::info;
use remotia::{field_vec, vec_avg};
use csv::Writer;

use crate::profiling::TransmissionRoundStats;

use super::TransmissionRoundLogger;

pub struct TransmissionRoundCSVLogger { 
}

impl TransmissionRoundLogger for TransmissionRoundCSVLogger {
    fn log(&self, round_stats: &TransmissionRoundStats) {
        
    }
}
