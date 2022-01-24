#![allow(dead_code)]

use log::debug;

use crate::common::feedback::FeedbackMessage;

use super::Encoder;

pub struct IdentityEncoder { }

impl IdentityEncoder {
    pub fn new() -> Self {
        Self { }
    }
}

impl Encoder for IdentityEncoder {
    fn encode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> usize {
        let encoded_frame_length = input_buffer.len();
        output_buffer.copy_from_slice(input_buffer);
        encoded_frame_length
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
