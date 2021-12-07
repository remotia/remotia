use std::{sync::Arc, time::Instant};

use bytes::BytesMut;
use log::{debug, info, warn};
use object_pool::{Pool, Reusable};
use tokio::{
    sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::{
    common::helpers::silo::channel_pull,
    server::{encode::Encoder, profiling::TransmittedFrameStats},
};

use super::capture::CaptureResult;

pub struct EncodeResult {
    pub capture_timestamp: u128,

    pub encoded_frame_buffer: BytesMut,
    pub frame_stats: TransmittedFrameStats,
}

pub fn launch_encode_thread(
    mut encoder: Box<dyn Encoder + Send>,
    raw_frame_buffers_sender: UnboundedSender<BytesMut>,
    mut encoded_frame_buffers_receiver: UnboundedReceiver<BytesMut>,
    mut capture_result_receiver: UnboundedReceiver<CaptureResult>,
    encode_result_sender: UnboundedSender<EncodeResult>,
    maximum_capture_delay: u128,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let (capture_result, capture_result_wait_time) =
                channel_pull(&mut capture_result_receiver)
                    .await
                    .expect("Capture channel closed, terminating.");

            let capture_delay = capture_result.capture_time.elapsed().as_millis();

            let raw_frame_buffer = capture_result.raw_frame_buffer;

            if capture_delay < maximum_capture_delay {
                let mut frame_stats = capture_result.frame_stats;

                let (mut encoded_frame_buffer, encoded_frame_buffer_wait_time) =
                    channel_pull(&mut encoded_frame_buffers_receiver)
                        .await
                        .expect("Encoded frame buffers channel closed, terminating.");

                let encoding_start_time = Instant::now();

                frame_stats.encoded_size =
                    encoder.encode(&raw_frame_buffer, &mut encoded_frame_buffer);

                frame_stats.encoding_time = encoding_start_time.elapsed().as_millis();
                frame_stats.encoder_idle_time =
                    capture_result_wait_time + encoded_frame_buffer_wait_time;
                frame_stats.capture_delay = capture_delay;

                let send_result = encode_result_sender.send(EncodeResult {
                    capture_timestamp: capture_result.capture_timestamp,
                    encoded_frame_buffer,
                    frame_stats,
                });

                if let Err(_) = send_result {
                    warn!("Transfer result sender error");
                    break;
                };
            } else {
                debug!("Dropping frame (capture delay: {})", capture_delay);
            }

            raw_frame_buffers_sender.send(raw_frame_buffer)
                .expect("Raw buffer return error");
        }
    })
}
