use crate::common::feedback::FeedbackMessage;

use crate::types::FrameData;

pub trait FrameCapturer {
    fn capture(&mut self, frame_data: &mut FrameData);

    fn handle_feedback(&mut self, message: FeedbackMessage);

    fn width(&self) -> usize;
    fn height(&self) -> usize;
}

