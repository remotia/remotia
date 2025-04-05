use std::{fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;

use crate::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

pub struct BuffersPool<K: Copy> {
    slot_id: K,
    buffers_sender: Sender<BytesMut>,
    buffers_receiver: Arc<Mutex<Receiver<BytesMut>>>,
}

impl<K: Copy> BuffersPool<K> {
    pub async fn new(slot_id: K, pool_size: usize, buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(pool_size);

        for _ in 0..pool_size {
            let buf = BytesMut::with_capacity(buffer_size);
            sender.send(buf).await.unwrap();
        }

        Self {
            slot_id,
            buffers_sender: sender,
            buffers_receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn borrower(&self) -> BufferBorrower<K> {
        BufferBorrower {
            slot_id: self.slot_id,
            receiver: self.buffers_receiver.clone(),
        }
    }

    pub fn redeemer(&self) -> BufferRedeemer<K> {
        BufferRedeemer {
            slot_id: self.slot_id.clone(),
            sender: self.buffers_sender.clone(),
            soft: false,
        }
    }
}

pub struct BufferBorrower<K> {
    slot_id: K,
    receiver: Arc<Mutex<Receiver<BytesMut>>>,
}

#[async_trait]
impl<F, K> FrameProcessor<F> for BufferBorrower<K>
where
    K: Copy + Debug + Send,
    F: PullableFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        log::debug!("Borrowing '{:?}' buffer...", self.slot_id);

        let mut receiver = self.receiver.lock().await;

        loop {
            match tokio::time::timeout(Duration::from_millis(1000), receiver.recv()).await {
                Ok(result) => {
                    if let Some(buffer) = result {
                        frame_data.push(self.slot_id, buffer);
                        break;
                    }
                }
                Err(err) => {
                    log::warn!("Unable to borrow '{:?}' buffer: {:?}", self.slot_id, err);
                }
            }
        }

        Some(frame_data)
    }
}

pub struct BufferRedeemer<K> {
    slot_id: K,
    sender: Sender<BytesMut>,
    soft: bool,
}

impl<K> BufferRedeemer<K> {
    pub fn soft(mut self) -> Self {
        self.soft = true;
        self
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for BufferRedeemer<K>
where
    K: Copy + Debug + Send,
    F: PullableFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        log::debug!(
            "Redeeming '{:?}' buffer (soft = {})...",
            self.slot_id, self.soft
        );

        let buffer = frame_data.pull(&self.slot_id);

        match buffer {
            Some(mut buffer) => {
                buffer.clear();

                self.sender
                    .send(buffer)
                    .await
                    .expect(&format!("Unable to redeem '{:?}' buffer", self.slot_id));

                if self.soft {
                    log::debug!("Soft-redeemed a '{:?}' buffer", self.slot_id);
                }
            }
            None => {
                if !self.soft {
                    panic!("Missing '{:?}' buffer", self.slot_id);
                }
            }
        }

        log::debug!("Redeemed '{:?}' buffer (soft = {})", self.slot_id, self.soft);

        Some(frame_data)
    }
}
