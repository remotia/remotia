use chrono::Utc;

use crate::client::{ClientConfiguration, profiling::{ReceptionRoundStats, logging::{ReceptionRoundLogger, console::ReceptionRoundConsoleLogger, csv::ReceptionRoundCSVLogger}}};

pub fn setup_round_stats(config: &ClientConfiguration) -> Result<ReceptionRoundStats, std::io::Error> {
    let round_stats: ReceptionRoundStats = {
        let datetime = Utc::now();

        ReceptionRoundStats {
            loggers: {
                let mut loggers: Vec<Box<dyn ReceptionRoundLogger>> = Vec::new();

                if config.csv_profiling  {
                    loggers.push(Box::new(ReceptionRoundCSVLogger::new(
                        format!("csv_logs/client/{}.csv", datetime).as_str(),
                    )?));
                }

                if config.console_profiling  {
                    loggers.push(Box::new(ReceptionRoundConsoleLogger::default()));
                }

                loggers
            },

            ..Default::default()
        }
    };
    Ok(round_stats)
}


