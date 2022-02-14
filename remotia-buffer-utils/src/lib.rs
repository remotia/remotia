use bytes::BytesMut;
use remotia::{traits::FrameProcessor, types::FrameData};

use async_trait::async_trait;

pub struct BufferAllocator { 
    buffer_id: String,
    size: usize
}

impl BufferAllocator {
    pub fn new(buffer_id: &str, size: usize) -> Self {
        Self {
            buffer_id: buffer_id.to_string(),
            size
        }
    }

    fn allocate_buffer(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(self.size);
        buf.resize(self.size, 0);
        buf
    }
}

#[async_trait]
impl FrameProcessor for BufferAllocator {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        frame_data.insert_writable_buffer(&self.buffer_id, self.allocate_buffer());
        Some(frame_data)
    }

}