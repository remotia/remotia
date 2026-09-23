use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use log::{debug, info, trace};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::traits::FrameProcessor;

/// Yields execution once without parking the task on the current worker's
/// defer queue.
///
/// `Component::launch` awaits this once per processed frame, for *every*
/// component: it bounds each Tokio poll to a single frame, so a backlogged
/// component hands the worker back to the scheduler instead of running for
/// its whole cooperative budget (~128 ops, i.e. tens of frames of synchronous
/// codec/PNG/render work) inside one poll while the tasks it wakes wait
/// behind it. Head components and drain iterations get the same per-frame
/// hop they always had; mid-pipeline components are the ones this newly
/// bounds.
///
/// The yield must wake the task immediately: `tokio::task::yield_now()`
/// (Tokio >= 1.53) hands the waker to the worker-local defer queue, which is
/// only drained once that worker runs out of ready tasks. A deferred task is
/// not visible to idle workers and resumes only after everything else on its
/// own worker — including a long synchronous processor (ONNX inference, PNG
/// encoding, ...) — has finished, serializing the pipeline. Waking the task
/// immediately instead makes it ready work, stealable by any worker right
/// away.
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

    tag: Option<String>,
}

unsafe impl<F> Send for Component<F> {}

impl<F: Default + Send + 'static> Component<F> {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            receiver: None,
            sender: None,
            shutdown_signal: None,
            tag: None,
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
                    info!(
                        "[{}] Drain complete and shutdown signaled, shutting down",
                        tag
                    );
                    break;
                }

                // One frame per poll: hop back to the scheduler (immediate
                // wake, see `fair_yield`) after every processed frame, so
                // backlogged chains cannot monopolize their worker for their
                // whole cooperative budget while the tasks they wake (downstream
                // components, pool borrowers) starve behind this poll. Placed
                // after the `break`s so shutdown iterations don't hop.
                fair_yield().await;
            }
        })
    }
}

impl<F: Default + Send + 'static> Default for Component<F> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use async_trait::async_trait;
    use tokio::sync::oneshot;

    use super::*;

    #[derive(Default)]
    struct TestFrame;

    /// Counts processed frames and notifies the test after the first one.
    /// Returns `None` on the fourth call, i.e. once the input channel is
    /// closed and the component enters drain mode, so the component exits.
    struct FrameCounter {
        processed: Arc<AtomicUsize>,
        first_seen: Option<oneshot::Sender<()>>,
    }

    #[async_trait]
    impl FrameProcessor<TestFrame> for FrameCounter {
        async fn process(&mut self, frame_data: TestFrame) -> Option<TestFrame> {
            let count = self.processed.fetch_add(1, Ordering::SeqCst) + 1;
            if count == 1 {
                if let Some(first_seen) = self.first_seen.take() {
                    let _ = first_seen.send(());
                }
            }
            (count <= 3).then_some(frame_data)
        }
    }

    /// Regression test for multi-frame polls: a component with queued frames
    /// must hand the worker back after *every* frame.
    ///
    /// The observer runs as a *spawned* task on purpose: spawned tasks share
    /// the scheduler's FIFO queue (`push_back`/`pop_front`) with the
    /// component, so it is polled between the component's polls — the
    /// `block_on` root future, in contrast, only runs once the task queue
    /// runs dry (or every `event_interval` ticks) and would miss the
    /// interleaving entirely. The observer is woken from inside the
    /// component's first frame, right before the component's own
    /// `fair_yield()` re-queues it, so the FIFO order is
    /// `[observer, component]`. Without the per-frame yield the component
    /// drains the whole queue inside a single poll and the observer sees
    /// three processed frames — the starvation bug where encode/sink chains
    /// ran ~128 cooperative ops deep per poll while the tasks they woke
    /// waited behind them.
    #[test]
    fn poll_is_bounded_to_one_frame() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        runtime.block_on(async {
            let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();
            for _ in 0..3 {
                input_tx.send(TestFrame).unwrap();
            }

            let processed = Arc::new(AtomicUsize::new(0));
            let (first_seen_tx, first_seen_rx) = oneshot::channel();

            let observer = tokio::spawn({
                let processed = processed.clone();
                async move {
                    first_seen_rx
                        .await
                        .expect("the first frame was never processed");
                    processed.load(Ordering::SeqCst)
                }
            });

            let mut component = Component::<TestFrame>::new().append(FrameCounter {
                processed: processed.clone(),
                first_seen: Some(first_seen_tx),
            });
            component.set_receiver(input_rx);
            let task = component.launch();

            let count = observer.await.expect("the observer panicked");
            assert_eq!(
                count, 1,
                "the component processed more than one frame in a single poll"
            );

            // Close the input: the component works through the two remaining
            // frames, its fourth processor call returns `None` in drain mode
            // and the component shuts down.
            drop(input_tx);
            task.await.expect("the component panicked");
        });
    }
}
