use crate::common::feedback::FeedbackMessage;

pub trait Renderer {
    fn render(&mut self, raw_frame_buffer: &[u8]);
    fn handle_feedback(&mut self, message: FeedbackMessage);
    fn get_buffer_size(&self) -> usize;
}