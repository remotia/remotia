use async_trait::async_trait;
use log::debug;
use remotia_core::{traits::FrameProcessor, types::FrameData};
use scrap::{Capturer, Display};

use core::slice;

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

    pub fn width(&self) -> usize {
        self.capturer.width()
    }

    pub fn height(&self) -> usize {
        self.capturer.height()
    }

    fn capture_on_frame_data(&mut self, frame_data: &mut FrameData) {
        debug!("Capturing...");

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
}

#[async_trait]
impl FrameProcessor for ScrapFrameCapturer {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        self.capture_on_frame_data(&mut frame_data);
        Some(frame_data)
    }
}
