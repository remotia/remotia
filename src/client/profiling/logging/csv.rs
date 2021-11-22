use std::{fs::File, io::Write};

use log::info;
use crate::{field_vec, vec_avg};
use csv::Writer;

use crate::client::profiling::{ReceptionRoundStats, ReceivedFrameStats};

use super::ReceptionRoundLogger;

pub struct ReceptionRoundCSVLogger { 
    writer: Writer<File>
}

impl ReceptionRoundCSVLogger {
    pub fn new(path: &str) -> Result<Self, csv::Error> {
        Ok(ReceptionRoundCSVLogger {
            writer: csv::Writer::from_path(path)?
        })
    }
}

impl ReceptionRoundLogger for ReceptionRoundCSVLogger {
    fn log(&mut self, profiled_frames: &Vec<ReceivedFrameStats>) {
        profiled_frames.iter().for_each(|frame| {
            self.writer.serialize(frame).unwrap();
        });

        self.writer.flush().unwrap();
    }
}

