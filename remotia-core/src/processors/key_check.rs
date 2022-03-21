use async_trait::async_trait;

use crate::{traits::FrameProcessor, types::FrameData};

pub struct KeyChecker {
    key: String
}

impl KeyChecker {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
        }
    }
}

#[async_trait]
impl FrameProcessor for KeyChecker {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        if frame_data.has(&self.key) {
            Some(frame_data)
        } else {
            None
        }
    }
}