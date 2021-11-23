use chrono::Utc;

use crate::server::{ServerConfiguration, profiling::{TransmissionRoundStats, logging::{TransmissionRoundLogger, console::TransmissionRoundConsoleLogger, csv::TransmissionRoundCSVLogger}}};

pub fn setup_round_stats(config: &ServerConfiguration) -> Result<TransmissionRoundStats, std::io::Error> {
    let round_stats: TransmissionRoundStats = {
        let datetime = Utc::now();

        TransmissionRoundStats {
            loggers: {
                let mut loggers: Vec<Box<dyn TransmissionRoundLogger>> = Vec::new();

                if config.csv_profiling  {
                    loggers.push(Box::new(TransmissionRoundCSVLogger::new(
                        format!("csv_logs/server/{}.csv", datetime).as_str(),
                    )?));
                }

                if config.console_profiling  {
                    loggers.push(Box::new(TransmissionRoundConsoleLogger::default()));
                }

                loggers
            },

            ..Default::default()
        }
    };
    Ok(round_stats)
}
