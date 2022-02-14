#![allow(dead_code)]

use crate::{error::DropReason, common::feedback::FeedbackMessage};

use async_trait::async_trait;

pub mod identity;
pub mod pool;

#[async_trait]
pub trait Decoder {
    async fn decode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> Result<usize, DropReason>;
    fn handle_feedback(&mut self, message: FeedbackMessage);
}