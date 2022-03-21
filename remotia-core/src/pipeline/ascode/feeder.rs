use tokio::sync::mpsc::UnboundedSender;

use crate::types::FrameData;

pub struct AscodePipelineFeeder {
    sender: UnboundedSender<FrameData>
}

impl AscodePipelineFeeder {
    pub fn new(sender: UnboundedSender<FrameData>) -> Self {
        Self {
            sender
        }
    }

    pub fn feed(&self, frame_data: FrameData) {
        self.sender.send(frame_data).unwrap();
    }
}