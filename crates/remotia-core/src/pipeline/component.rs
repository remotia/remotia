use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};

use log::{debug, info, trace};
use tokio::{
    task::JoinHandle, sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{traits::FrameProcessor};

/// Yields execution once without parking the task on the current worker's
/// defer queue.
///
/// `tokio::task::yield_now()` (Tokio >= 1.53) hands the waker to the
/// worker-local defer queue, which is only drained once that worker runs out
/// of ready tasks. A pipeline head component shares its worker with long
/// synchronous processors (ONNX inference, PNG encoding, ...), so the deferred
/// head is not scheduled again until the whole downstream chain has finished,
/// serializing the pipeline. Waking the task immediately instead makes it
/// visible to the other runtime workers right away.
async fn fair_yield() {
    struct YieldOnce(bool);

    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    YieldOnce(false).await
}

pub struct Component<F> {
    processors: Vec<Box<dyn FrameProcessor<F> + Send>>,

    receiver: Option<UnboundedReceiver<F>>,
    sender: Option<UnboundedSender<F>>,

    shutdown_signal: Option<Arc<AtomicBool>>,

    tag: Option<String>
}

unsafe impl<F> Send for Component<F> {}

impl<F: Default + Send + 'static> Component<F> {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            receiver: None,
            sender: None,
            shutdown_signal: None,
            tag: None
        }
    }

    pub fn singleton<T: 'static + FrameProcessor<F> + Send>(processor: T) -> Self {
        Self::new().append(processor)
    }

    pub fn append<T: 'static + FrameProcessor<F> + Send>(mut self, processor: T) -> Self {
        self.processors.push(Box::new(processor));
        self
    }

    pub fn tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    //////////////////////
    // Internal methods //
    //////////////////////

    pub(crate) fn set_sender(&mut self, sender: UnboundedSender<F>) {
        self.sender = Some(sender);
    }

    pub(crate) fn set_receiver(&mut self, receiver: UnboundedReceiver<F>) {
        self.receiver = Some(receiver);
    }

    pub(crate) fn set_shutdown_signal(&mut self, signal: Arc<AtomicBool>) {
        self.shutdown_signal = Some(signal);
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown_signal
            .as_ref()
            .map(|s| s.load(Ordering::Relaxed))
            .unwrap_or(false)
    }

    pub(crate) fn launch(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut channel_closed = false;

            loop {
                let mut frame_data = if let Some(receiver) = self.receiver.as_mut() {
                    if channel_closed {
                        fair_yield().await;
                        Some(F::default())
                    } else {
                        match receiver.recv().await {
                            Some(frame) => Some(frame),
                            None => {
                                let tag = self.tag.as_deref().unwrap_or("");
                                trace!("[{}] Receive channel closed, entering drain mode", tag);
                                channel_closed = true;
                                Some(F::default())
                            }
                        }
                    }
                } else if self.is_shutdown() {
                    let tag = self.tag.as_deref().unwrap_or("");
                    info!("[{}] Shutdown signal received, shutting down", tag);
                    break;
                } else {
                    fair_yield().await;
                    Some(F::default())
                };

                for processor in &mut self.processors {
                    frame_data = processor.process(frame_data.unwrap()).await;

                    if frame_data.is_none() {
                        break;
                    }
                }

                let is_none = frame_data.is_none();

                if let Some(sender) = self.sender.as_mut() {
                    if let Some(frame_data) = frame_data {
                        if sender.send(frame_data).is_err() {
                            let tag = self.tag.as_deref().unwrap_or("");
                            info!("[{}] Send channel closed, shutting down", tag);
                            break;
                        }
                    }
                }

                if channel_closed && is_none {
                    let tag = self.tag.as_deref().unwrap_or("");
                    info!("[{}] Drain complete and shutdown signaled, shutting down", tag);
                    break;
                }
            }
        })
    }
}


impl<F: Default + Send + 'static> Default for Component<F> {
    fn default() -> Self {
        Self::new()
    }
}
