use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;

use crate::BytesMut;
use remotia_core::traits::{FrameProcessor, PullableFrameProperties};
use tokio::sync::{
    mpsc::{self, error::TryRecvError, Receiver, Sender},
    Mutex,
};

/// A fixed-size pool of reusable [`BytesMut`] buffers.
///
/// Borrowing blocks while the pool is empty: the hard
/// [`BufferBorrower`] parks on the pool channel (holding the receiver's
/// mutex) until a redeemer sends a buffer back, which wakes it immediately —
/// waiting costs no CPU and the waiter is plain ready work, schedulable on
/// any runtime worker.
///
/// A pool expects a single borrower: a second borrower would queue (FIFO) on
/// the receiver's mutex behind the parked one rather than race it for
/// buffers. If every redeemer (and the pool itself) is dropped, the parked
/// borrower is woken with a `Disconnected` error and panics instead of
/// waiting for a buffer that can never arrive.
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
            soft: false,
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
    soft: bool,
}

impl<K> BufferBorrower<K> {
    pub fn soft(mut self) -> Self {
        self.soft = true;
        self
    }
}

#[async_trait]
impl<F, K> FrameProcessor<F> for BufferBorrower<K>
where
    K: Copy + Debug + Send,
    F: PullableFrameProperties<K, BytesMut> + Send + 'static,
{
    async fn process(&mut self, mut frame_data: F) -> Option<F> {
        let mut receiver = self.receiver.lock().await;

        if self.soft {
            // Soft borrowers never wait: they take a free buffer if there is
            // one and let the frame through without a slot otherwise.
            return match receiver.try_recv() {
                Ok(buffer) => {
                    frame_data.push(self.slot_id, buffer);
                    Some(frame_data)
                }
                Err(err) => {
                    log::debug!("Unable to borrow '{:?}' buffer: {:?}", self.slot_id, err);
                    Some(frame_data)
                }
            };
        }

        let buffer = match receiver.try_recv() {
            Ok(buffer) => Some(buffer),
            Err(TryRecvError::Empty) => {
                log::debug!("Waiting for '{:?}' buffer...", self.slot_id);
                // Park until a redeemer sends a buffer back: the send wakes
                // this task immediately, so the waiter is ready work again
                // instead of a `yield_now` spin that only resumes once its
                // own worker drains its defer queue. `None` means every
                // sender (redeemer) is gone.
                receiver.recv().await
            }
            Err(TryRecvError::Disconnected) => None,
        }
        .unwrap_or_else(|| {
            panic!(
                "Unable to borrow '{:?}' buffer: every redeemer of this pool is gone",
                self.slot_id
            )
        });

        frame_data.push(self.slot_id, buffer);
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
        log::trace!(
            "Redeeming '{:?}' buffer (soft = {})...",
            self.slot_id,
            self.soft
        );

        let buffer = frame_data.pull(&self.slot_id);

        match buffer {
            Some(mut buffer) => {
                buffer.clear();

                // `expect(&format!(..))` would allocate a String on every
                // redeem (twice per frame on the server, thrice on the
                // client); the panic message is built only on the error path.
                self.sender
                    .send(buffer)
                    .await
                    .unwrap_or_else(|_| panic!("Unable to redeem '{:?}' buffer", self.slot_id));

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

        log::trace!(
            "Redeemed '{:?}' buffer (soft = {})",
            self.slot_id,
            self.soft
        );

        Some(frame_data)
    }
}
