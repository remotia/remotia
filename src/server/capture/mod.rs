use crate::common::feedback::FeedbackMessage;

pub mod scrap;

pub trait FrameCapturer {
    fn capture(&mut self) -> Result<&[u8], std::io::Error>;

    fn handle_feedback(&mut self, message: FeedbackMessage);

    fn width(&self) -> usize;
    fn height(&self) -> usize;
}

