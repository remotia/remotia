use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use log::{debug, warn};
use object_pool::{Pool, Reusable};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::common::helpers::silo::channel_pull;
use crate::server::profiling::TransmittedFrameStats;
use crate::server::send::FrameSender;

use super::encode::EncodeResult;

pub struct TransferResult {
    pub frame_stats: TransmittedFrameStats,
}

pub fn launch_transfer_thread(
    mut frame_sender: Box<dyn FrameSender + Send>,
    encoded_frame_buffers_sender: UnboundedSender<BytesMut>,
    mut encode_result_receiver: UnboundedReceiver<EncodeResult>,
    transfer_result_sender: UnboundedSender<TransferResult>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let (encode_result, encode_result_wait_time) =
                channel_pull(&mut encode_result_receiver)
                    .await
                    .expect("Encode result channel closed, terminating.");

            let encoded_frame_buffer = encode_result.encoded_frame_buffer;
            let mut frame_stats = encode_result.frame_stats;

            let transfer_start_time = Instant::now();

            frame_stats.transmitted_bytes = frame_sender
                .send_frame(encode_result.capture_timestamp, &encoded_frame_buffer[..frame_stats.encoded_size])
                .await;

            encoded_frame_buffers_sender.send(encoded_frame_buffer)
                .expect("Encoded frame buffer return error");

            frame_stats.transfer_time = transfer_start_time.elapsed().as_millis();
            frame_stats.transferrer_idle_time = encode_result_wait_time;

            let send_result = transfer_result_sender
                .send(TransferResult { frame_stats });

            if let Err(_) = send_result {
                warn!("Transfer result sender error");
                break;
            };
        }
    })
}
