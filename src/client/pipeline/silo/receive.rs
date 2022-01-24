use std::ops::ControlFlow;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use bytes::BytesMut;
use log::{debug, info, warn};
use tokio::sync::broadcast;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::client::error::ClientError;
use crate::client::profiling::ReceivedFrameStats;
use crate::client::receive::{FrameReceiver, ReceivedFrame};
use crate::common::feedback::FeedbackMessage;
use crate::common::helpers::silo::channel_pull;

pub struct ReceiveResult {
    pub received_frame: Option<ReceivedFrame>,
    pub encoded_frame_buffer: BytesMut,

    pub frame_stats: ReceivedFrameStats,
}

pub fn launch_receive_thread(
    mut frame_receiver: Box<dyn FrameReceiver + Send>,
    mut encoded_frame_buffers_receiver: UnboundedReceiver<BytesMut>,
    receive_result_sender: UnboundedSender<ReceiveResult>,
    mut feedback_receiver: broadcast::Receiver<FeedbackMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            pull_feedback(&mut feedback_receiver, &mut frame_receiver);

            let (mut encoded_frame_buffer, encoded_frame_buffer_wait_time) =
                pull_buffer(&mut encoded_frame_buffers_receiver).await;

            let (receive_result, reception_time) =
                receive(&mut frame_receiver, &mut encoded_frame_buffer).await;

            let reception_delay = calculate_reception_delay(&receive_result);

            let (received_frame, error) = match receive_result {
                Ok(received_frame) => (Some(received_frame), None),
                Err(err) => (None, Some(err)),
            };

            let mut frame_stats = initialize_frame_stats(
                reception_time,
                reception_delay,
                encoded_frame_buffer_wait_time,
                error,
            );

            let received_frame = extract_received_frame(received_frame, &mut frame_stats);

            if let ControlFlow::Break(_) = push_result(
                &receive_result_sender,
                received_frame,
                encoded_frame_buffer,
                frame_stats,
            ) {
                break;
            }
        }
    })
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

fn push_result(
    receive_result_sender: &UnboundedSender<ReceiveResult>,
    received_frame: Option<ReceivedFrame>,
    encoded_frame_buffer: BytesMut,
    frame_stats: ReceivedFrameStats,
) -> ControlFlow<()> {
    debug!("Sending result...");
    let send_result = receive_result_sender.send(ReceiveResult {
        received_frame,
        encoded_frame_buffer,
        frame_stats,
    });
    if let Err(e) = send_result {
        warn!("Receive result send error: {}", e);
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

fn extract_received_frame(
    received_frame: Option<ReceivedFrame>,
    frame_stats: &mut ReceivedFrameStats,
) -> Option<ReceivedFrame> {
    let received_frame = if received_frame.is_some() {
        let received_frame = received_frame.unwrap();
        frame_stats.capture_timestamp = received_frame.capture_timestamp;

        Some(received_frame)
    } else {
        None
    };
    received_frame
}

fn initialize_frame_stats(
    reception_time: u128,
    reception_delay: u128,
    encoded_frame_buffer_wait_time: u128,
    error: Option<ClientError>,
) -> ReceivedFrameStats {
    let mut frame_stats = ReceivedFrameStats::default();
    frame_stats.reception_time = reception_time;
    frame_stats.reception_delay = reception_delay;
    frame_stats.receiver_idle_time = encoded_frame_buffer_wait_time;
    frame_stats.error = error;
    frame_stats
}

fn calculate_reception_delay(receive_result: &Result<ReceivedFrame, ClientError>) -> u128 {
    let reception_delay = if receive_result.is_ok() {
        let received_frame = receive_result.as_ref().unwrap();
        received_frame.reception_delay
    } else {
        0
    };
    reception_delay
}

async fn receive(
    frame_receiver: &mut Box<dyn FrameReceiver + Send>,
    encoded_frame_buffer: &mut BytesMut,
) -> (Result<ReceivedFrame, ClientError>, u128) {
    debug!("Receiving...");
    let reception_start_time = Instant::now();
    let receive_result = frame_receiver
        .receive_encoded_frame(encoded_frame_buffer)
        .await;
    let reception_time = reception_start_time.elapsed().as_millis();
    debug!("Received");
    (receive_result, reception_time)
}

fn pull_feedback(
    feedback_receiver: &mut broadcast::Receiver<FeedbackMessage>,
    frame_receiver: &mut Box<dyn FrameReceiver + Send>,
) {
    match feedback_receiver.try_recv() {
        Ok(message) => {
            frame_receiver.handle_feedback(message);
        }
        Err(_) => {}
    };
}
