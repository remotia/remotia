use chrono::Utc;

use crate::client::profiling::{ReceptionRoundStats, logging::{console::ReceptionRoundConsoleLogger, csv::ReceptionRoundCSVLogger}};

pub fn setup_round_stats() -> Result<ReceptionRoundStats, std::io::Error> {
    let round_stats: ReceptionRoundStats = {
        let datetime = Utc::now();

        ReceptionRoundStats {
            loggers: vec![
                Box::new(ReceptionRoundCSVLogger::new(
                    format!("csv_logs/client/{}.csv", datetime).as_str(),
                )?),
                Box::new(ReceptionRoundConsoleLogger::default()),
            ],

            ..Default::default()
        }
    };
    Ok(round_stats)
}


