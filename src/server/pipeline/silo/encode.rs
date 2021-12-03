use std::{sync::Arc, time::Instant};

use bytes::BytesMut;
use log::{debug, info, warn};
use object_pool::{Pool, Reusable};
use tokio::{
    sync::mpsc::{Receiver, UnboundedSender, UnboundedReceiver},
    task::JoinHandle,
};

use crate::server::{encode::Encoder, profiling::TransmittedFrameStats};

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
            debug!("Encoding...");

            let capture_result_wait_start_time = Instant::now();
            let capture_result = capture_result_receiver.recv().await;
            let capture_result_wait_time = capture_result_wait_start_time.elapsed().as_millis();

            if capture_result.is_none() {
                debug!("Capture channel has been closed, terminating");
                break;
            }
            let capture_result = capture_result.unwrap();

            let capture_delay = capture_result.capture_time.elapsed().as_millis();

            let raw_frame_buffer = capture_result.raw_frame_buffer;

            if capture_delay < maximum_capture_delay {
                let mut frame_stats = capture_result.frame_stats;

                let encoded_frame_buffer_wait_start_time = Instant::now();
                let encoded_frame_buffer = encoded_frame_buffers_receiver.recv().await;
                let encoded_frame_buffer_wait_time =
                    encoded_frame_buffer_wait_start_time.elapsed().as_millis();

                if encoded_frame_buffer.is_none() {
                    debug!("Raw frame buffers channel closed, terminating.");
                    break;
                }
                let mut encoded_frame_buffer = encoded_frame_buffer.unwrap();

                let encoding_start_time = Instant::now();

                frame_stats.encoded_size =
                    encoder.encode(&raw_frame_buffer, &mut encoded_frame_buffer);

                frame_stats.encoding_time = encoding_start_time.elapsed().as_millis();
                frame_stats.encoder_idle_time =
                    capture_result_wait_time + encoded_frame_buffer_wait_time;
                frame_stats.capture_delay = capture_delay;

                let send_result = encode_result_sender
                    .send(EncodeResult {
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

            let buffer_return_result = raw_frame_buffers_sender.send(raw_frame_buffer);
            if let Err(_) = buffer_return_result {
                warn!("Buffer return error");
                break;
            };
        }
    })
}
