use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use bytes::BytesMut;
use log::{debug, warn};
use object_pool::{Pool, Reusable};
use tokio::sync::broadcast;
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::common::feedback::FeedbackMessage;
use crate::common::helpers::silo::channel_pull;
use crate::server::send::FrameSender;
use crate::types::FrameData;

use super::encode::EncodeResult;
use super::utils::return_writable_buffer;

pub struct TransferResult {
    pub frame_data: FrameData,
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

            let mut frame_data = encode_result.frame_data;

            if frame_data.get_drop_reason().is_none() {
                transfer(&mut frame_sender, &mut frame_data).await;

                frame_data.set("transferrer_encode_result_wait_time", encode_result_wait_time);
                frame_data.set(
                    "total_time",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        - frame_data.get("capture_timestamp"),
                );
            } else {
                debug!("Error on encoded frame: {:?}", frame_data.get_drop_reason());
            }

            return_writable_buffer(
                &encoded_frame_buffers_sender,
                &mut frame_data,
                "encoded_frame_buffer",
            );

            let send_result = transfer_result_sender.send(TransferResult { frame_data });
            if let Err(_) = send_result {
                warn!("Transfer result sender error");
                break;
            };
        }
    })
}

async fn transfer(
    frame_sender: &mut Box<dyn FrameSender + Send>,
    frame_data: &mut FrameData,
) {
    debug!("Transmitting...");
    let transfer_start_time = Instant::now();
    frame_sender.send_frame(frame_data).await;

    frame_data.set("transfer_time", transfer_start_time.elapsed().as_millis());
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
