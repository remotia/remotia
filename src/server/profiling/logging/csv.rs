use std::{fs::File, io::Write};

use csv::Writer;

use crate::server::profiling::TransmittedFrameStats;

use super::TransmissionRoundLogger;

pub struct TransmissionRoundCSVLogger { 
    writer: Writer<File>
}

impl TransmissionRoundCSVLogger {
    pub fn new(path: &str) -> Result<Self, csv::Error> {
        Ok(TransmissionRoundCSVLogger {
            writer: csv::Writer::from_path(path)?
        })
    }
}

impl TransmissionRoundLogger for TransmissionRoundCSVLogger {
    fn log(&mut self, profiled_frames: &Vec<TransmittedFrameStats>) {
        profiled_frames.iter().for_each(|frame| {
            self.writer.serialize(frame).unwrap();
        });

        self.writer.flush().unwrap();
    }
}
