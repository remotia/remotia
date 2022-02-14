use bytes::{Bytes, BytesMut};

use crate::common::feedback::FeedbackMessage;

use crate::error::DropReason;
use crate::types::FrameData;

use async_trait::async_trait;

pub mod identity;
pub mod pool;

#[async_trait]
pub trait Encoder {
    async fn encode(&mut self, frame_data: &mut FrameData);
    fn handle_feedback(&mut self, message: FeedbackMessage);
}