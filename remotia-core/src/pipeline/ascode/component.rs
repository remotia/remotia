use std::time::Duration;

use log::{debug, info};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::{traits::FrameProcessor, types::FrameData};

pub struct Component {
    processors: Vec<Box<dyn FrameProcessor + Send>>,

    receiver: Option<UnboundedReceiver<FrameData>>,
    sender: Option<UnboundedSender<FrameData>>,
}

unsafe impl Send for Component {}

impl Component {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            receiver: None,
            sender: None,
        }
    }

    pub fn append<T: 'static + FrameProcessor + Send>(mut self, processor: T) -> Self {
        self.processors.push(Box::new(processor));
        self
    }

    //////////////////////
    // Internal methods //
    //////////////////////

    pub(crate) fn set_sender(&mut self, sender: UnboundedSender<FrameData>) {
        self.sender = Some(sender);
    }

    pub(crate) fn set_receiver(&mut self, receiver: UnboundedReceiver<FrameData>) {
        self.receiver = Some(receiver);
    }

    pub(crate) fn launch(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let mut frame_data = if self.receiver.is_some() {
                    Some(
                        self.receiver
                            .as_mut()
                            .unwrap()
                            .recv()
                            .await
                            .expect("Receive channel closed"),
                    )
                } else {
                    debug!("No receiver registered, allocating an empty frame DTO");
                    Some(FrameData::default())
                };

                debug!("Received frame data: {}", frame_data.as_ref().unwrap());

                for processor in &mut self.processors {
                    frame_data = processor.process(frame_data.unwrap()).await;

                    if frame_data.is_none() {
                        break;
                    }
                }

                if self.sender.is_some() {
                    if let Some(frame_data) = frame_data {
                        debug!("Sending frame data: {}", frame_data);
                        if self.sender.as_mut().unwrap().send(frame_data).is_err() {
                            panic!("Error while sending frame data");
                        }
                    }
                }
            }
        })
    }
}

impl Default for Component {
    fn default() -> Self {
        Self::new()
    }
}
