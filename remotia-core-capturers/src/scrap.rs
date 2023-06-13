use async_trait::async_trait;
use log::debug;
use remotia_core::traits::{FrameProcessor, BorrowMutFrameProperties};
use scrap::{Capturer, Display};
use remotia_buffer_utils::{BufMut, BytesMut};

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
            capturer,
        }
    }

    pub fn new_from_primary(buffer_key: K) -> Self {
        let display = Display::primary().expect("Couldn't find primary display.");
        let capturer = Capturer::new(display).expect("Couldn't begin capture.");
        Self {
            buffer_key,
            capturer,
        }
    }

    pub fn width(&self) -> usize {
        self.capturer.width()
    }

    pub fn height(&self) -> usize {
        self.capturer.height()
    }

    pub fn buffer_size(&mut self) -> usize {
        self.capturer.frame().unwrap().len()
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for ScrapFrameCapturer<K>
where
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        debug!("Capturing...");
        let output_buffer = frame_data.get_mut_ref(&self.buffer_key).unwrap();
        match self.capturer.frame() {
            Ok(buffer) => {
                let frame_slice = unsafe { slice::from_raw_parts(buffer.as_ptr(), buffer.len()) };
                output_buffer.put(frame_slice);
            }
            Err(error) => {
                panic!("Scrap capture error: {}", error);
            }
        }
        Some(frame_data)
    }
}
