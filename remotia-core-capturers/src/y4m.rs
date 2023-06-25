use std::fs::File;

use async_trait::async_trait;
use log::debug;
use remotia_buffer_utils::{BufMut, BytesMut};
use remotia_core::traits::{BorrowMutFrameProperties, FrameProcessor};
use y4m::Decoder;

pub struct Y4MFrameCapturer<K> {
    stream: Decoder<File>,
    buffer_key: K,
}

impl<K> Y4MFrameCapturer<K> {
    pub fn new(buffer_key: K, path: &str) -> Self {
        Self {
            stream: y4m::decode(File::open(path).unwrap()).unwrap(),
            buffer_key,
        }
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for Y4MFrameCapturer<K>
where
    K: Send,
    F: BorrowMutFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let frame = self.stream.read_frame();
        if frame.is_err() {
            debug!("No more frames to extract");
            return None;
        }

        let frame = frame.unwrap();

        let buffer = frame_data.get_mut_ref(&self.buffer_key).unwrap();
        buffer.put(frame.get_y_plane());
        buffer.put(frame.get_u_plane());
        buffer.put(frame.get_v_plane());

        Some(frame_data)
    }
}
