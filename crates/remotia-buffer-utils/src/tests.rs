use std::collections::HashMap;

use bytes::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};

use crate::BufferAllocator;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum BufferType {
    Test,
}

#[derive(Default)]
struct TestFrameData {
    buffers: HashMap<BufferType, BytesMut>,
}

impl PullableFrameProperties<BufferType, BytesMut> for TestFrameData {
    fn push(&mut self, key: BufferType, value: BytesMut) {
        self.buffers.insert(key, value);
    }

    fn pull(&mut self, key: &BufferType) -> Option<BytesMut> {
        self.buffers.remove(key)
    }
}

#[tokio::test]
async fn test_allocation() {
    let mut allocator = BufferAllocator::new(BufferType::Test, 1024);
    let mut dto = TestFrameData::default();
    dto = allocator.process(dto).await.unwrap();
    assert!(dto.pull(&BufferType::Test).is_some());
}
