use std::{fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;

use crate::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

mod buffers;
pub use buffers::*;

const DEFAULT_TIMEOUT: u64 = 1000;


pub struct AutodropBuffersPool<K: Copy> {
    slot_id: K,
    buffers_receiver: Arc<Mutex<Receiver<AutodroppingBuffer>>>,
    max_timeout: u64,
}

impl<K: Copy> AutodropBuffersPool<K> {
    pub async fn new(slot_id: K, pool_size: usize, buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel::<AutodroppingBuffer>(pool_size);

        for _ in 0..pool_size {
            let buffer = AutodroppingBuffer {
                data: BytesMut::with_capacity(buffer_size),
                sender: sender.clone(),
            };
            sender.send(buffer).await.unwrap();
        }

        Self {
            slot_id,
            buffers_receiver: Arc::new(Mutex::new(receiver)),
            max_timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn set_timeout(mut self, value: u64) -> Self {
        self.max_timeout = value;
        self
    }

    pub fn borrower(&self) -> AutodropBufferBorrower<K> {
        AutodropBufferBorrower {
            slot_id: self.slot_id,
            receiver: self.buffers_receiver.clone(),
            max_timeout: self.max_timeout,
        }
    }
}

pub struct AutodropBufferBorrower<K> {
    slot_id: K,
    receiver: Arc<Mutex<Receiver<AutodroppingBuffer>>>,
    max_timeout: u64,
}

#[async_trait]
impl<F, K> FrameProcessor<F> for AutodropBufferBorrower<K>
where
    K: Copy + Debug + Send,
    F: PullableFrameProperties<K, AutodroppingBuffer> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        log::debug!("Borrowing '{:?}' buffer...", self.slot_id);

        let mut receiver = self.receiver.lock().await;

        loop {
            match tokio::time::timeout(Duration::from_millis(self.max_timeout), receiver.recv())
                .await
            {
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
