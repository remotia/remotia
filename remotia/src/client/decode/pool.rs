use log::debug;

use crate::error::DropReason;
use async_trait::async_trait;

use super::Decoder;

pub struct PoolDecoder {
    decoders: Vec<Box<dyn Decoder + Send>>,
}

unsafe impl Send for PoolDecoder {}

impl PoolDecoder {
    pub fn new(decoders: Vec<Box<dyn Decoder + Send>>) -> Self {
        Self { decoders }
    }
}

#[async_trait]
impl Decoder for PoolDecoder {
    async fn decode(
        &mut self,
        input_buffer: &[u8],
        output_buffer: &mut [u8],
    ) -> Result<usize, DropReason> {
        debug!("{:?}", &input_buffer[0..8]);

        let chosen_decoder_index = input_buffer[0] as usize;
        let encoded_frame_buffer = &input_buffer[1..];

        debug!(
            "Decoding {} bytes with decoder #{}...",
            encoded_frame_buffer.len(),
            chosen_decoder_index
        );

        let chosen_decoder = &mut self.decoders[chosen_decoder_index];

        let result = chosen_decoder.decode(encoded_frame_buffer, output_buffer).await;

        result
    }

    fn handle_feedback(&mut self, message: crate::common::feedback::FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
