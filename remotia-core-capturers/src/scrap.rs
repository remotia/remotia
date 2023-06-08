use async_trait::async_trait;
use bytes::BytesMut;
use log::debug;
use remotia_core::{traits::{FrameProcessor, BorrowableFrameProperties}};
use scrap::{Capturer, Display};

use core::slice;

pub struct ScrapFrameCapturer<K> {
    buffer_key: K,
    capturer: Capturer,
}

// TODO: Evaluate a safer way to move the capturer to another thread
// Necessary for multi-threaded pipelines
unsafe impl<K> Send for ScrapFrameCapturer<K> {}

impl<K> ScrapFrameCapturer<K> {
    pub fn new(buffer_key: K, capturer: Capturer) -> Self {
        Self { 
            buffer_key,
            capturer 
        }
    }

    pub fn new_from_primary(buffer_key: K) -> Self {
        let display = Display::primary().expect("Couldn't find primary display.");
        let capturer = Capturer::new(display).expect("Couldn't begin capture.");
        Self { buffer_key, capturer }
    }

    pub fn width(&self) -> usize {
        self.capturer.width()
    }

    pub fn height(&self) -> usize {
        self.capturer.height()
    }

    pub fn capture_in_buffer(&mut self, dest: &mut [u8]) {
        debug!("Capturing...");
        match self.capturer.frame() {
            Ok(buffer) => {
                let frame_slice = unsafe { slice::from_raw_parts(buffer.as_ptr(), buffer.len()) };
                dest.copy_from_slice(frame_slice);
            }
            Err(error) => {
                panic!("Scrap capture error: {}", error);
            }
        }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for ScrapFrameCapturer<K> where
    F: BorrowableFrameProperties<K, BytesMut> + Send + 'static
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let buffer_ref = frame_data.get_mut_ref(&self.buffer_key).unwrap();
        self.capture_in_buffer(buffer_ref);
        Some(frame_data)
    }
}
