use bytes::BytesMut;
use remotia_core::{traits::{FrameProcessor, FrameProperties}};

use async_trait::async_trait;

// pub mod pool;
// pub mod pool_registry;

#[cfg(test)]
mod tests;
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
impl<F> FrameProcessor<F> for BufferAllocator 
    where F: FrameProperties<BytesMut> + Send + 'static
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        frame_data.set(&self.buffer_id, self.allocate_buffer());
        Some(frame_data)
    }
}