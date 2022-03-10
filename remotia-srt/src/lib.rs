use std::collections::HashMap;

use remotia::types::FrameData;
use serde::{Deserialize, Serialize};

pub mod receiver;
pub mod sender;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub(crate) struct SRTFrameData {
    encoded_frame_buffer: Vec<u8>,
    stats: HashMap<String, u128>,
}

impl SRTFrameData {
    pub fn from_frame_data(frame_data: &mut FrameData) -> Self {
        let encoded_size = frame_data.get("encoded_size") as usize;

        let encoded_frame_buffer = frame_data
            .get_writable_buffer_ref("encoded_frame_buffer")
            .unwrap();

        let encoded_frame_buffer = (encoded_frame_buffer[..encoded_size]).to_vec();

        Self {
            stats: frame_data.get_stats().clone(),
            encoded_frame_buffer,
        }
    }

    pub fn merge_with_frame_data(self, frame_data: &mut FrameData) {
        frame_data.merge_stats(self.stats);

        let encoded_frame_buffer = frame_data
            .get_writable_buffer_ref("encoded_frame_buffer")
            .unwrap();

        let encoded_size = self.encoded_frame_buffer.len();
        encoded_frame_buffer[..encoded_size].copy_from_slice(&self.encoded_frame_buffer)
    }
}
