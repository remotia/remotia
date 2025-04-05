use remotia_core::traits::{FrameProcessor, PullableFrameProperties};

use async_trait::async_trait;

pub mod pool;
pub mod pool_registry;

pub use bytes::*;


#[cfg(test)]
mod tests;
pub struct BufferAllocator<K> {
    buffer_key: K,
    size: usize,
}

impl<K> BufferAllocator<K> {
    pub fn new(buffer_key: K, size: usize) -> Self {
        Self { buffer_key, size }
    }

    fn allocate_buffer(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(self.size);
        buf.resize(self.size, 0);
        buf
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for BufferAllocator<K>
where
    F: PullableFrameProperties<K, BytesMut> + Send + 'static,
    K: Copy + Send,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        frame_data.push(self.buffer_key, self.allocate_buffer());
        Some(frame_data)
    }
}
