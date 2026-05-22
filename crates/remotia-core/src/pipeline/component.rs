use std::time::Duration;

use log::{debug, info};
use tokio::{
    task::JoinHandle, sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{traits::FrameProcessor};

macro_rules! tagged {
    ($self:ident, $msg:tt) => {{
        &format!("[{}] {}", $self.tag.as_ref().unwrap_or(&"".to_string()), $msg)
    }}
}

pub struct Component<F> {
    processors: Vec<Box<dyn FrameProcessor<F> + Send>>,

    receiver: Option<UnboundedReceiver<F>>,
    sender: Option<UnboundedSender<F>>,

    tag: Option<String>
}

unsafe impl<F> Send for Component<F> {}

impl<F: Default + Send + 'static> Component<F> {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            receiver: None,
            sender: None,
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

    pub(crate) fn launch(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let mut frame_data = if self.receiver.is_some() {
                    match self.receiver.as_mut().unwrap().recv().await {
                        Some(frame) => Some(frame),
                        None => {
                            let tag = self.tag.as_deref().unwrap_or("");
                            info!("[{}] Receive channel closed, shutting down", tag);
                            break;
                        }
                    }
                } else {
                    debug!("No receiver registered, allocating an empty frame DTO");
                    Some(F::default())
                };

                for processor in &mut self.processors {
                    frame_data = processor.process(frame_data.unwrap()).await;

                    if frame_data.is_none() {
                        break;
                    }
                }

                if self.sender.is_some() {
                    if let Some(frame_data) = frame_data {
                        if self.sender.as_mut().unwrap().send(frame_data).is_err() {
                            let tag = self.tag.as_deref().unwrap_or("");
                            info!("[{}] Send channel closed, shutting down", tag);
                            break;
                        }
                    }
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
