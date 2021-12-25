use std::time::{Instant, SystemTime, UNIX_EPOCH};

use bytes::BytesMut;
use log::{debug, info, warn};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::client::error::ClientError;
use crate::client::profiling::ReceivedFrameStats;
use crate::client::receive::{FrameReceiver, ReceivedFrame};
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
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            debug!("Pulling empty encoded frame buffer...");

            let (mut encoded_frame_buffer, encoded_frame_buffer_wait_time) =
                channel_pull(&mut encoded_frame_buffers_receiver)
                    .await
                    .expect("Encoded frame buffers channel closed, terminating.");

            debug!("Receiving...");
            let reception_start_time = Instant::now();
            let receive_result = frame_receiver
                .receive_encoded_frame(&mut encoded_frame_buffer)
                .await;
            let reception_time = reception_start_time.elapsed().as_millis();
            debug!("Received");

            let reception_delay = if receive_result.is_ok() {
                let received_frame = receive_result.as_ref().unwrap();
                received_frame.reception_delay
            } else {
                0
            };

            let (received_frame, error) = match receive_result {
                Ok(received_frame) => (Some(received_frame), None),
                Err(err) => (None, Some(err)),
            };

            let mut frame_stats = ReceivedFrameStats::default();
            frame_stats.reception_time = reception_time;
            frame_stats.reception_delay = reception_delay;
            frame_stats.receiver_idle_time = encoded_frame_buffer_wait_time;
            frame_stats.error = error;

            let received_frame = if received_frame.is_some() {
                let received_frame = received_frame.unwrap();
                frame_stats.capture_timestamp = received_frame.capture_timestamp;

                Some(received_frame)
            } else {
                None
            };

            debug!("Sending result...");
            let send_result = receive_result_sender.send(ReceiveResult {
                received_frame,
                encoded_frame_buffer,
                frame_stats,
            });

            if let Err(e) = send_result {
                warn!("Receive result send error: {}", e);
                break;
            };
        }
    })
}
