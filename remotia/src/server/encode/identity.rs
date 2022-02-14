#![allow(dead_code)]

use bytes::{Bytes, BytesMut};
use log::debug;

use crate::{common::feedback::FeedbackMessage, error::DropReason, traits::FrameProcessor, types::FrameData};

use async_trait::async_trait;

use super::Encoder;

pub struct IdentityEncoder {}

impl IdentityEncoder {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl FrameProcessor for IdentityEncoder {
    async fn process(&mut self, mut frame_data: FrameData) -> Option<FrameData> {
        let input_buffer = frame_data.extract_writable_buffer("raw_frame_buffer").unwrap();
        let mut output_buffer = frame_data.extract_writable_buffer("encoded_frame_buffer").unwrap();

        output_buffer.copy_from_slice(&input_buffer);
        frame_data.set("encoded_size", input_buffer.len() as u128);

        frame_data.insert_writable_buffer("raw_frame_buffer", input_buffer);
        frame_data.insert_writable_buffer("encoded_frame_buffer", output_buffer);

        Some(frame_data)
    }
}
