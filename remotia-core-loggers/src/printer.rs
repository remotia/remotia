use async_trait::async_trait;

use log::info;
use remotia_core::{traits::FrameProcessor, types::FrameData};

pub struct ConsoleFrameDataPrinter {}

impl ConsoleFrameDataPrinter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ConsoleFrameDataPrinter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FrameProcessor for ConsoleFrameDataPrinter {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        info!("{}", frame_data);
        Some(frame_data)
    }
}
