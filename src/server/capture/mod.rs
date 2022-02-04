use crate::common::feedback::FeedbackMessage;

use super::types::ServerFrameData;

pub mod scrap;

pub trait FrameCapturer {
    fn capture(&mut self, frame_data: &mut ServerFrameData);

    fn handle_feedback(&mut self, message: FeedbackMessage);

    fn width(&self) -> usize;
    fn height(&self) -> usize;
}

