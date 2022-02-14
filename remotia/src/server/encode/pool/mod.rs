#![allow(dead_code)]

use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use log::debug;
use tokio::sync::{
    mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender},
    Mutex,
};

use async_trait::async_trait;

use super::Encoder;
use crate::{
    common::feedback::FeedbackMessage,
    error::DropReason,
    types::FrameData,
};

const POOLING_INFO_SIZE: usize = 1;

struct EncodingResult {
    encoding_unit: EncodingUnit,
    frame_data: FrameData,
}

struct EncodingUnit {
    id: u8,
    encoder: Box<dyn Encoder + Send>,
    cached_raw_frame_buffer: BytesMut,
    cached_encoded_frame_buffer: BytesMut,
}
unsafe impl Send for EncodingUnit {}

pub struct PoolEncoder {
    encoding_units: Vec<EncodingUnit>,
    encoded_frames_sender: UnboundedSender<EncodingResult>,
    encoded_frames_receiver: UnboundedReceiver<EncodingResult>,
}

unsafe impl Send for PoolEncoder {}

impl PoolEncoder {
    pub fn new(buffers_size: usize, mut encoders: Vec<Box<dyn Encoder + Send>>) -> Self {
        let (encoded_frames_sender, encoded_frames_receiver) =
            mpsc::unbounded_channel::<EncodingResult>();

        let mut encoding_units = Vec::new();
        let mut i = 0;

        while encoders.len() > 0 {
            let encoder = encoders.pop().unwrap();
            encoding_units.push(EncodingUnit {
                id: i,
                encoder,
                cached_raw_frame_buffer: Self::allocate_buffer(buffers_size),
                cached_encoded_frame_buffer: Self::allocate_buffer(buffers_size),
            });

            i += 1;
        }

        Self {
            encoding_units,
            encoded_frames_sender,
            encoded_frames_receiver,
        }
    }

    fn allocate_buffer(size: usize) -> BytesMut {
        let mut buf = BytesMut::with_capacity(size);
        buf.resize(size, 0);
        buf
    }

    fn push_to_unit(&mut self, frame_data: &mut FrameData, mut unit: EncodingUnit) {
        let encoder_id = unit.id;
        let frame_id = frame_data.get("capture_timestamp");
        debug!("Pushing frame #{} to encoder #{}...", frame_id, encoder_id);

        let result_sender = self.encoded_frames_sender.clone();

        // Clone the pipeline's raw frame buffer
        let raw_frame_buffer = frame_data
            .get_writable_buffer_ref("raw_frame_buffer")
            .unwrap();
        unit.cached_raw_frame_buffer
            .copy_from_slice(raw_frame_buffer);

        // Clone the DTO excluding buffers, whom should be returned to the pipeline
        let mut local_frame_data = frame_data.clone_without_buffers();

        // Launch the encoding task
        tokio::spawn(async move {
            // Split buffers such that memory is owned by frame data
            // Reserve space for pooling info on the encoded frame buffer
            let local_raw_frame_buffer = unit.cached_raw_frame_buffer.split();
            local_frame_data.insert_writable_buffer("raw_frame_buffer", local_raw_frame_buffer);

            let local_encoded_frame_buffer = unit
                .cached_encoded_frame_buffer
                .split_off(POOLING_INFO_SIZE);
            local_frame_data
                .insert_writable_buffer("encoded_frame_buffer", local_encoded_frame_buffer);

            unit.encoder.encode(&mut local_frame_data).await;

            debug!("Sending encoder #{} result...", unit.id);
            let send_result = result_sender.send(EncodingResult {
                encoding_unit: unit,
                frame_data: local_frame_data,
            });

            if send_result.is_err() {
                panic!("Unhandled pool encoder result channel error on send");
            }
        });
    }
}

#[async_trait]
impl Encoder for PoolEncoder {
    async fn encode(&mut self, frame_data: &mut FrameData) {
        // Push
        let chosen_encoding_unit = self.encoding_units.pop();
        let available_encoders = chosen_encoding_unit.is_some();

        if available_encoders {
            let chosen_encoding_unit = chosen_encoding_unit.unwrap();
            self.push_to_unit(frame_data, chosen_encoding_unit);
        }

        // Pull
        let encoding_result;

        if available_encoders {
            let pull_result = self.encoded_frames_receiver.try_recv();

            if let Err(TryRecvError::Empty) = pull_result {
                debug!("No encoding results");
                frame_data.set_drop_reason(Some(DropReason::NoEncodedFrames));
                return;
            }

            encoding_result = pull_result.unwrap();
        } else {
            let pull_result = self.encoded_frames_receiver.recv().await;

            encoding_result = pull_result.unwrap();
        }

        let mut encoding_unit = encoding_result.encoding_unit;
        let mut local_frame_data = encoding_result.frame_data;

        // Copy the local encoded frame into the pipeline's buffer
        let encoded_frame_buffer = frame_data
            .get_writable_buffer_ref("encoded_frame_buffer")
            .unwrap();

        let local_encoded_frame_buffer = local_frame_data
            .get_writable_buffer_ref("encoded_frame_buffer")
            .unwrap();

        encoded_frame_buffer[0] = encoding_unit.id;
        encoded_frame_buffer[POOLING_INFO_SIZE..].copy_from_slice(local_encoded_frame_buffer);

        // Return memory ownership to the encoding unit
        encoding_unit.cached_raw_frame_buffer.unsplit(
            local_frame_data
                .extract_writable_buffer("raw_frame_buffer")
                .unwrap(),
        );

        encoding_unit.cached_encoded_frame_buffer.unsplit(
            local_frame_data
                .extract_writable_buffer("encoded_frame_buffer")
                .unwrap(),
        );

        // Insert frame stats
        let capture_timestamp = local_frame_data.get("capture_timestamp");
        let capture_time = local_frame_data.get("capture_time");
        frame_data.set("capture_timestamp", capture_timestamp);
        frame_data.set("capture_time", capture_time);

        debug!(
            "Frame #{} size: {} encoder: #{}",
            capture_timestamp,
            local_frame_data.get("encoded_size"),
            encoding_unit.id
        );

        self.encoding_units.push(encoding_unit);

        frame_data.set(
            "encoded_size",
            local_frame_data.get("encoded_size") + POOLING_INFO_SIZE as u128,
        );
    }

    fn handle_feedback(&mut self, message: FeedbackMessage) {
        debug!("Feedback message: {:?}", message);
    }
}
