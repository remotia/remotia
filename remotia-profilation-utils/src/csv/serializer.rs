use std::fs::File;

use async_trait::async_trait;

use csv::Writer;
use remotia_core::{traits::FrameProcessor, types::FrameData};

pub struct CSVFrameDataSerializer {
    writer: Writer<File>,

    values_to_log: Vec<String>,

    columns_written: bool,
    log_drop_reason: bool,
}

impl CSVFrameDataSerializer {
    pub fn new(path: &str) -> Self {
        let prefix = std::path::Path::new(path).parent().unwrap();
        std::fs::create_dir_all(prefix).unwrap();

        Self {
            writer: csv::Writer::from_path(path).unwrap(),
            values_to_log: Vec::new(),
            columns_written: false,
            log_drop_reason: false,
        }
    }

    pub fn log(mut self, value: &str) -> Self {
        self.values_to_log.push(value.to_string());
        self
    }

    pub fn log_drop_reason(mut self) -> Self {
        self.log_drop_reason = true;
        self
    }
}

#[async_trait]
impl FrameProcessor for CSVFrameDataSerializer {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        if !self.columns_written {
            let mut columns = self.values_to_log.clone();
            if self.log_drop_reason {
                columns.push("drop_reason".to_string());
            }
            self.writer
                .write_record(columns)
                .unwrap();
            self.columns_written = true;
        }

        let mut record: Vec<String> = self
            .values_to_log
            .iter()
            .map(|key| 
                frame_data.get_opt(key)
                    .map(|value| format!("{}", value))
                    .unwrap_or("".to_string())
            ).collect();

        if self.log_drop_reason {
            record.push(frame_data.get_drop_reason()
                .map(|drop_reason| drop_reason.to_string())
                .unwrap_or("".to_string()));
        }

        self.writer.write_record(record).unwrap();
        self.writer.flush().unwrap();

        Some(frame_data)
    }
}
