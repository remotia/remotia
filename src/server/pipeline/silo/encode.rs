use std::{ops::ControlFlow, sync::Arc, time::Instant};

use bytes::{Bytes, BytesMut};
use log::{debug, info, warn};
use object_pool::{Pool, Reusable};
use tokio::{
    sync::{
        broadcast::{error::TryRecvError, Receiver},
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    task::JoinHandle,
};

use crate::{
    common::{feedback::FeedbackMessage, helpers::silo::channel_pull},
    server::{encode::Encoder, error::ServerError, types::ServerFrameData},
};

use super::{capture::CaptureResult, utils::return_writable_buffer};

pub struct EncodeResult {
    pub frame_data: ServerFrameData,
}

pub fn launch_encode_thread(
    mut encoder: Box<dyn Encoder + Send>,
    raw_frame_buffers_sender: UnboundedSender<BytesMut>,
    mut encoded_frame_buffers_receiver: UnboundedReceiver<BytesMut>,
    mut capture_result_receiver: UnboundedReceiver<CaptureResult>,
    encode_result_sender: UnboundedSender<EncodeResult>,
    mut feedback_receiver: Receiver<FeedbackMessage>,
    maximum_capture_delay: u128,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            pull_feedback(&mut feedback_receiver, &mut encoder);

            let (capture_result, capture_result_wait_time) =
                pull_capture_result(&mut capture_result_receiver).await;

            let capture_delay = capture_result.capture_time.elapsed().as_millis();
            let fresh_frame = capture_delay < maximum_capture_delay;
            let mut frame_data = capture_result.frame_data;

            if fresh_frame {
                let (encoded_frame_buffer, encoded_frame_buffer_wait_time) =
                    pull_buffer(&mut encoded_frame_buffers_receiver).await;

                frame_data.insert_writable_buffer("encoded_frame_buffer", encoded_frame_buffer);
                frame_data.set_local("encoder_capture_result_wait_time", capture_result_wait_time);
                frame_data.set_local(
                    "encoder_encoded_frame_buffer_wait_time",
                    encoded_frame_buffer_wait_time,
                );
                frame_data.set_local("capture_delay", capture_delay);

                encode(&mut encoder, &mut frame_data).await;
            } else {
                debug!("Dropping frame (capture delay: {})", capture_delay);
            }

            return_writable_buffer(
                &raw_frame_buffers_sender,
                &mut frame_data,
                "raw_frame_buffer",
            );

            if fresh_frame {
                if let ControlFlow::Break(_) =
                    push_result(&encode_result_sender, EncodeResult { frame_data })
                {
                    break;
                }
            }
        }
    })
}

fn push_result(
    encode_result_sender: &UnboundedSender<EncodeResult>,
    result: EncodeResult,
) -> ControlFlow<()> {
    let encode_result_sender = encode_result_sender;
    debug!("Pushing encode result...");
    let send_result = encode_result_sender.send(result);
    if let Err(_) = send_result {
        warn!("Transfer result sender error");
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

async fn encode(encoder: &mut Box<dyn Encoder + Send>, frame_data: &mut ServerFrameData) {
    let encoding_start_time = Instant::now();
    encoder.encode(frame_data).await;
    let encoding_time = encoding_start_time.elapsed().as_millis();

    frame_data.set_local("encoding_time", encoding_time);
}

async fn pull_buffer(
    encoded_frame_buffers_receiver: &mut UnboundedReceiver<BytesMut>,
) -> (BytesMut, u128) {
    debug!("Pulling empty encoded frame buffer...");
    let (encoded_frame_buffer, encoded_frame_buffer_wait_time) =
        channel_pull(encoded_frame_buffers_receiver)
            .await
            .expect("Encoded frame buffers channel closed, terminating.");
    (encoded_frame_buffer, encoded_frame_buffer_wait_time)
}

async fn pull_capture_result(
    capture_result_receiver: &mut UnboundedReceiver<CaptureResult>,
) -> (CaptureResult, u128) {
    debug!("Pulling capture result...");

    let (capture_result, capture_result_wait_time) = channel_pull(capture_result_receiver)
        .await
        .expect("Capture channel closed, terminating.");
    (capture_result, capture_result_wait_time)
}

fn pull_feedback(
    feedback_receiver: &mut Receiver<FeedbackMessage>,
    encoder: &mut Box<dyn Encoder + Send>,
) {
    debug!("Pulling feedback...");
    match feedback_receiver.try_recv() {
        Ok(message) => {
            encoder.handle_feedback(message);
        }
        Err(_) => {}
    };
}
