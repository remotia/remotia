use chrono::Utc;

use crate::server::profiling::{TransmissionRoundStats, logging::{console::TransmissionRoundConsoleLogger, csv::TransmissionRoundCSVLogger}};

pub fn setup_round_stats() -> Result<TransmissionRoundStats, std::io::Error> {
    let round_stats: TransmissionRoundStats = {
        let datetime = Utc::now();

        TransmissionRoundStats {
            loggers: vec![
                Box::new(TransmissionRoundCSVLogger::new(
                    format!("csv_logs/server/{}.csv", datetime).as_str(),
                )?),
                Box::new(TransmissionRoundConsoleLogger::default()),
            ],

            ..Default::default()
        }
    };
    Ok(round_stats)
}
