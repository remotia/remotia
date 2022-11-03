use std::{sync::{Arc}, time::Duration};

use async_trait::async_trait;

use bytes::BytesMut;
use log::{debug, trace, warn};
use remotia_core::{traits::FrameProcessor, types::FrameData, error::DropReason};
use tokio::sync::{Mutex, mpsc::{Sender, Receiver, self}};

pub struct BuffersPool {
    slot_id: String,
    buffers_sender: Sender<BytesMut>,
    buffers_receiver: Arc<Mutex<Receiver<BytesMut>>>,
}

impl BuffersPool {
    pub async fn new(slot_id: &str, pool_size: usize, buffer_size: usize) -> Self {
        let slot_id = slot_id.to_string();

        let (sender, receiver) = mpsc::channel(pool_size);

        for _ in 0..pool_size {
            let mut buf = BytesMut::with_capacity(buffer_size);
            buf.resize(buffer_size, 0);
            sender.send(buf).await.unwrap();
        }

        Self { slot_id, 
            buffers_sender: sender,
            buffers_receiver: Arc::new(Mutex::new(receiver))
        }
    }

    pub fn borrower(&self) -> BufferBorrower {
        BufferBorrower {
            slot_id: self.slot_id.clone(),
            receiver: self.buffers_receiver.clone()
        }
    }

    pub fn redeemer(&self) -> BufferRedeemer {
        BufferRedeemer {
            slot_id: self.slot_id.clone(),
            sender: self.buffers_sender.clone(),
            soft: false,
        }
    }
}

pub struct BufferBorrower {
    slot_id: String,
    receiver: Arc<Mutex<Receiver<BytesMut>>>
}

#[async_trait]
impl FrameProcessor for BufferBorrower {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        debug!("Borrowing '{}' buffer...", self.slot_id);
        let mut receiver = self.receiver.lock().await;

        loop {
            match tokio::time::timeout(Duration::from_millis(1000), receiver.recv()).await {
                Ok(result) => {
                    if let Some(buffer) = result {
                        frame_data.insert_writable_buffer(&self.slot_id, buffer);
                        break;
                    }
                },
                Err(err) => {
                    warn!("Unable to borrow '{}' buffer: {:?}", self.slot_id, err);
                },
            }
        }

        Some(frame_data)
    }
}

pub struct BufferRedeemer {
    slot_id: String,
    sender: Sender<BytesMut>,
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
                self.sender.send(buffer).await.expect("Unable to redeem buffer");

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
