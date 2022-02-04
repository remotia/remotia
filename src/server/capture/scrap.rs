use log::debug;
use scrap::{Capturer, Display, Frame};

use core::slice;
use std::io::ErrorKind::WouldBlock;

use crate::{common::feedback::FeedbackMessage, server::types::ServerFrameData};

use super::FrameCapturer;

pub struct ScrapFrameCapturer {
    capturer: Capturer,
}

// TODO: Evaluate a safer way to move the capturer to another thread
// Necessary for multi-threaded pipelines
unsafe impl Send for ScrapFrameCapturer {}

impl ScrapFrameCapturer {
    pub fn new(capturer: Capturer) -> Self {
        Self { capturer }
    }

    pub fn new_from_primary() -> Self {
        let display = Display::primary().expect("Couldn't find primary display.");
        let capturer = Capturer::new(display).expect("Couldn't begin capture.");
        Self { capturer }
    }
}

impl FrameCapturer for ScrapFrameCapturer {
    fn capture(&mut self, frame_data: &mut ServerFrameData) {
        match self.capturer.frame() {
            Ok(buffer) => {
                let frame_slice = unsafe { slice::from_raw_parts(buffer.as_ptr(), buffer.len()) };
                frame_data
                    .get_writable_buffer_ref("raw_frame_buffer")
                    .unwrap()
                    .copy_from_slice(frame_slice);
            }
            Err(error) => {
                panic!("Scrap capture error: {}", error);
            }
        }
    }

    fn width(&self) -> usize {
        self.capturer.width()
    }

    fn height(&self) -> usize {
        self.capturer.height()
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
