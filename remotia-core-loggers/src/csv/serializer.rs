use std::fs::File;

use async_trait::async_trait;

use csv::Writer;
use remotia::{traits::FrameProcessor, types::FrameData};

pub struct CSVFrameDataSerializer {
    writer: Writer<File>,

    values_to_log: Vec<String>,

    columns_written: bool,
}

impl CSVFrameDataSerializer {
    pub fn new(path: &str) -> Self {
        Self {
            writer: csv::Writer::from_path(path).unwrap(),
            values_to_log: Vec::new(),
            columns_written: false,
        }
    }

    pub fn log(mut self, value: &str) -> Self {
        self.values_to_log.push(value.to_string());
        self
    }
}

#[async_trait]
impl FrameProcessor for CSVFrameDataSerializer {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        if !self.columns_written {
            self.writer
                .write_record(self.values_to_log.clone())
                .unwrap();
            self.columns_written = true;
        }

        let record = self
            .values_to_log
            .iter()
            .map(|key| format!("{}", frame_data.get(key)));

        self.writer.write_record(record).unwrap();
        self.writer.flush().unwrap();

        Some(frame_data)
    }
}
