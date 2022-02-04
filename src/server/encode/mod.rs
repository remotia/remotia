use bytes::{Bytes, BytesMut};

use crate::common::feedback::FeedbackMessage;

use super::{error::ServerError, types::ServerFrameData};

use async_trait::async_trait;

// pub mod identity;
pub mod ffmpeg;
pub mod pool;

#[async_trait]
pub trait Encoder {
    async fn encode(&mut self, frame_data: &mut ServerFrameData);
    fn handle_feedback(&mut self, message: FeedbackMessage);
}