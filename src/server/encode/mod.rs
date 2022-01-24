use crate::common::feedback::FeedbackMessage;

pub mod identity;
pub mod ffmpeg;

mod utils;

pub trait Encoder {
    fn encode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> usize;
    fn handle_feedback(&mut self, message: FeedbackMessage);
}