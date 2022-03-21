use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

use async_trait::async_trait;

use log::debug;
use remotia_core::{traits::FrameProcessor, types::FrameData};

pub struct RawFrameDumper {
    buffer_id: String,

    key: String,

    folder: PathBuf,
}

impl RawFrameDumper {
    pub fn new(buffer_id: &str, folder: PathBuf) -> Self {
        create_dir_all(folder.clone()).unwrap();
        Self {
            buffer_id: buffer_id.to_string(),
            key: "capture_timestamp".to_string(),
            folder,
        }
    }

    pub fn key(mut self, key: &str) -> Self {
        self.key = key.to_string();
        self
    }
}

#[async_trait]
impl FrameProcessor for RawFrameDumper {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let frame_id = frame_data.get(&self.key);
        let buffer = frame_data.get_writable_buffer_ref(&self.buffer_id).unwrap();

        debug!("Dumping frame {}", frame_id);

        let mut file_path = self.folder.clone();
        file_path.push(format!("{}.bgra", frame_id));
        let mut output_file = File::create(file_path.as_path()).unwrap();
        output_file.write_all(buffer).unwrap();

        Some(frame_data)
    }
}
