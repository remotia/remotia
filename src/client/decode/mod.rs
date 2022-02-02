#![allow(dead_code)]

use crate::{client::error::ClientError, common::feedback::FeedbackMessage};

use async_trait::async_trait;

pub mod identity;
pub mod h264;
pub mod h264rgb;
pub mod h265;
pub mod pool;

mod utils;

#[async_trait]
pub trait Decoder {
    async fn decode(&mut self, input_buffer: &[u8], output_buffer: &mut [u8]) -> Result<usize, ClientError>;
    fn handle_feedback(&mut self, message: FeedbackMessage);
}