use chrono::Utc;

use crate::server::{profiling::{TransmissionRoundStats, logging::{TransmissionRoundLogger, console::TransmissionRoundConsoleLogger, csv::TransmissionRoundCSVLogger}}};

pub fn setup_round_stats(csv_profiling: bool, console_profiling: bool) -> Result<TransmissionRoundStats, std::io::Error> {
    let round_stats: TransmissionRoundStats = {
        let datetime = Utc::now();

        TransmissionRoundStats {
            loggers: {
                let mut loggers: Vec<Box<dyn TransmissionRoundLogger>> = Vec::new();

                if csv_profiling  {
                    loggers.push(Box::new(TransmissionRoundCSVLogger::new(
                        format!("csv_logs/server/{}.csv", datetime).as_str(),
                    )?));
                }

                if console_profiling  {
                    loggers.push(Box::new(TransmissionRoundConsoleLogger::default()));
                }

                loggers
            },

            ..Default::default()
        }
    };
    Ok(round_stats)
}
