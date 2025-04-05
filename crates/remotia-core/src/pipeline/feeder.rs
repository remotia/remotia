use std::fmt::Debug;

use tokio::sync::mpsc::UnboundedSender;

pub struct PipelineFeeder<F> {
    sender: UnboundedSender<F>
}

impl<F: Debug> PipelineFeeder<F> {
    pub fn new(sender: UnboundedSender<F>) -> Self {
        Self {
            sender
        }
    }

    pub fn feed(&self, frame_data: F) {
        self.sender.send(frame_data).unwrap();
    }
}