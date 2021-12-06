use chrono::Utc;

use crate::client::{profiling::{ReceptionRoundStats, logging::{ReceptionRoundLogger, console::ReceptionRoundConsoleLogger, csv::ReceptionRoundCSVLogger}}};

pub fn setup_round_stats(csv_profiling: bool, console_profiling: bool) -> Result<ReceptionRoundStats, std::io::Error> {
    let round_stats: ReceptionRoundStats = {
        let datetime = Utc::now();

        ReceptionRoundStats {
            loggers: {
                let mut loggers: Vec<Box<dyn ReceptionRoundLogger + Send>> = Vec::new();

                if csv_profiling  {
                    loggers.push(Box::new(ReceptionRoundCSVLogger::new(
                        format!("csv_logs/client/{}.csv", datetime).as_str(),
                    )?));
                }

                if console_profiling  {
                    loggers.push(Box::new(ReceptionRoundConsoleLogger::default()));
                }

                loggers
            },

            ..Default::default()
        }
    };
    Ok(round_stats)
}


