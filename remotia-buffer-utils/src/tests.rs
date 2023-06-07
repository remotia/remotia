use std::collections::HashMap;

use bytes::BytesMut;
use remotia_core::traits::{FrameProcessor, FrameProperties};

use crate::BufferAllocator;

#[derive(Default)]
struct TestFrameData {
    buffers: HashMap<String, BytesMut>
}

impl FrameProperties<BytesMut> for TestFrameData {
    fn set(&mut self, key: &str, value: BytesMut) {
        self.buffers.insert(key.to_string(), value); 
    }

    fn get(&mut self, key: &str) -> Option<BytesMut> {
        match self.buffers.get(key) {
            Some(buffer_ref) => Some(buffer_ref.clone()),
            None => None,
        }
    }
}

#[tokio::test]
async fn test_allocation() {
    let mut allocator = BufferAllocator::new("test_buffer", 1024);
    let mut dto = TestFrameData::default();
    dto = allocator.process(dto).await.unwrap();
    assert!(dto.get("test_buffer").is_some());
}
