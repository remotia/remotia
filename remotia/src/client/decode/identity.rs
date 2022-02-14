use log::debug;

use crate::{
    common::feedback::FeedbackMessage, error::DropReason, traits::FrameProcessor, types::FrameData,
};

use async_trait::async_trait;

use super::Decoder;

pub struct IdentityDecoder {}

impl IdentityDecoder {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl FrameProcessor for IdentityDecoder {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let input_buffer = frame_data
            .extract_writable_buffer("encoded_frame_buffer")
            .unwrap();
        let mut output_buffer = frame_data
            .extract_writable_buffer("raw_frame_buffer")
            .unwrap();

        output_buffer.copy_from_slice(&input_buffer);

        frame_data.insert_writable_buffer("encoded_frame_buffer", input_buffer);
        frame_data.insert_writable_buffer("raw_frame_buffer", output_buffer);

        Some(frame_data)
    }
}

#[async_trait]
impl Decoder for IdentityDecoder {
    async fn decode(
        &mut self,
        input_buffer: &[u8],
        output_buffer: &mut [u8],
    ) -> Result<usize, DropReason> {
        output_buffer.copy_from_slice(input_buffer);

        Ok(output_buffer.len())
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
