use std::{sync::{Arc}, time::Duration};

use async_trait::async_trait;

use bytes::BytesMut;
use log::debug;
use remotia_core::{error::DropReason, traits::FrameProcessor, types::FrameData};
use tokio::sync::Mutex;

pub struct BuffersPool {
    slot_id: String,
    buffers: Arc<Mutex<Vec<BytesMut>>>,
}

impl BuffersPool {
    pub fn new(slot_id: &str, pool_size: usize, buffer_size: usize) -> Self {
        let slot_id = slot_id.to_string();

        let mut buffers = Vec::new();

        for _ in 0..pool_size {
            let mut buf = BytesMut::with_capacity(buffer_size);
            buf.resize(buffer_size, 0);
            buffers.push(buf)
        }

        let buffers = Arc::new(Mutex::new(buffers));

        Self { slot_id, buffers }
    }

    pub fn borrower(&self) -> BufferBorrower {
        BufferBorrower {
            slot_id: self.slot_id.clone(),
            buffers: self.buffers.clone(),
            blocking: true
        }
    }

    pub fn redeemer(&self) -> BufferRedeemer {
        BufferRedeemer {
            slot_id: self.slot_id.clone(),
            buffers: self.buffers.clone(),
            soft: false,
        }
    }
}

pub struct BufferBorrower {
    slot_id: String,
    buffers: Arc<Mutex<Vec<BytesMut>>>,

    blocking: bool
}

impl BufferBorrower {
    pub fn blocking(mut self, blocking: bool) -> Self {
        self.blocking = blocking;
        self
    }
}

#[async_trait]
impl FrameProcessor for BufferBorrower {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        loop {
            debug!("Borrowing '{}' buffer...", self.slot_id);
            let mut buffers = self.buffers.lock().await;

            if let Some(buffer) = buffers.pop() {
                frame_data.insert_writable_buffer(&self.slot_id, buffer);
                break;
            } else {
                debug!("No available '{}' buffers", self.slot_id);
                if self.blocking {
                    drop(buffers);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                } else {
                    frame_data.set_drop_reason(Some(DropReason::NoAvailableBuffers))
                }
            }
        }

        Some(frame_data)
    }
}

pub struct BufferRedeemer {
    slot_id: String,
    buffers: Arc<Mutex<Vec<BytesMut>>>,
    soft: bool,
}

impl BufferRedeemer {
    pub fn soft(mut self) -> Self {
        self.soft = true;
        self
    }
}

#[async_trait]
impl FrameProcessor for BufferRedeemer {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        debug!("Redeeming '{}' buffer (soft = {})...", self.slot_id, self.soft);

        let buffer = frame_data.extract_writable_buffer(&self.slot_id);

        match buffer {
            Some(buffer) => {
                self.buffers.lock().await.push(buffer);
                if self.soft {
                    debug!("Soft-redeemed a '{}' buffer", self.slot_id);
                }
            },
            None => {
                if !self.soft {
                    panic!("Missing '{}' buffer in frame {}", self.slot_id, frame_data);
                }
            }
        }

        Some(frame_data)
    }
}
