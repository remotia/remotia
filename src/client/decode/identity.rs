use log::debug;

use crate::{client::error::ClientError, common::feedback::FeedbackMessage};

use async_trait::async_trait;

use super::Decoder;

pub struct IdentityDecoder {
    decoded_frame_buffer: Vec<u8>
}

impl IdentityDecoder {
    pub fn new(width: usize, height: usize) -> Self {
        let frame_buffer_size = width * height * 3;

        IdentityDecoder {
            decoded_frame_buffer: vec![0 as u8; frame_buffer_size]
        }
    }
}

#[async_trait]
impl Decoder for IdentityDecoder {
    async fn decode(
        &mut self,
        input_buffer: &[u8],
        output_buffer: &mut [u8],
    ) -> Result<usize, ClientError> {
        output_buffer.copy_from_slice(input_buffer);

        Ok(output_buffer.len())
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}

