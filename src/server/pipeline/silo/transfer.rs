use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use log::{debug, warn};
use object_pool::{Pool, Reusable};
use tokio::sync::broadcast;
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::common::feedback::FeedbackMessage;
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
    mut feedback_receiver: broadcast::Receiver<FeedbackMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            pull_feedback(&mut feedback_receiver, &mut frame_sender);

            let (encode_result, encode_result_wait_time) =
                pull_encode_result(&mut encode_result_receiver).await;

            let encoded_frame_buffer = encode_result.encoded_frame_buffer;
            let mut frame_stats = encode_result.frame_stats;

            if frame_stats.error.is_none() {
                let capture_timestamp = encode_result.capture_timestamp;

                let transfer_start_time = transfer(
                    &mut frame_stats,
                    &mut frame_sender,
                    capture_timestamp,
                    &encoded_frame_buffer,
                )
                .await;

                update_transfer_stats(
                    &mut frame_stats,
                    transfer_start_time,
                    encode_result_wait_time,
                );
            } else {
                debug!("Error on encoded frame: {:?}", frame_stats.error);
            }

            return_buffer(&encoded_frame_buffers_sender, encoded_frame_buffer);

            if let ControlFlow::Break(_) = push_result(&transfer_result_sender, frame_stats) {
                break;
            }
        }
    })
}

fn update_transfer_stats(
    frame_stats: &mut TransmittedFrameStats,
    transfer_start_time: Instant,
    encode_result_wait_time: u128,
) {
    frame_stats.transfer_time = transfer_start_time.elapsed().as_millis();
    frame_stats.transferrer_idle_time = encode_result_wait_time;
}

fn push_result(
    transfer_result_sender: &UnboundedSender<TransferResult>,
    frame_stats: TransmittedFrameStats,
) -> ControlFlow<()> {
    let send_result = transfer_result_sender.send(TransferResult { frame_stats });
    if let Err(_) = send_result {
        warn!("Transfer result sender error");
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

fn return_buffer(
    encoded_frame_buffers_sender: &UnboundedSender<BytesMut>,
    encoded_frame_buffer: BytesMut,
) {
    debug!("Returning empty encoded frame buffer...");
    encoded_frame_buffers_sender
        .send(encoded_frame_buffer)
        .expect("Encoded frame buffer return error");
}

async fn transfer(
    frame_stats: &mut TransmittedFrameStats,
    frame_sender: &mut Box<dyn FrameSender + Send>,
    capture_timestamp: u128,
    encoded_frame_buffer: &BytesMut,
) -> Instant {
    debug!("Transmitting...");
    let transfer_start_time = Instant::now();
    frame_stats.transmitted_bytes = frame_sender
        .send_frame(
            capture_timestamp,
            &encoded_frame_buffer[..frame_stats.encoded_size],
        )
        .await;
    transfer_start_time
}

async fn pull_encode_result(
    encode_result_receiver: &mut UnboundedReceiver<EncodeResult>,
) -> (EncodeResult, u128) {
    debug!("Pulling encode result...");
    let (encode_result, encode_result_wait_time) = channel_pull(encode_result_receiver)
        .await
        .expect("Encode result channel closed, terminating.");
    (encode_result, encode_result_wait_time)
}

fn pull_feedback(
    feedback_receiver: &mut broadcast::Receiver<FeedbackMessage>,
    frame_sender: &mut Box<dyn FrameSender + Send>,
) {
    debug!("Pulling feedback...");
    match feedback_receiver.try_recv() {
        Ok(message) => {
            frame_sender.handle_feedback(message);
        }
        Err(_) => {}
    };
}
