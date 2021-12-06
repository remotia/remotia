use std::time::{Instant, SystemTime, UNIX_EPOCH};

use bytes::BytesMut;
use log::{debug, warn};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

use crate::client::error::ClientError;
use crate::client::profiling::ReceivedFrameStats;
use crate::client::receive::{FrameReceiver, ReceivedFrame};

pub struct ReceiveResult {
    pub received_frame: ReceivedFrame,
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
            let encoded_frame_buffer_wait_start_time = Instant::now();
            let encoded_frame_buffer = encoded_frame_buffers_receiver.recv().await;
            let encoded_frame_buffer_wait_time =
                encoded_frame_buffer_wait_start_time.elapsed().as_millis();

            if encoded_frame_buffer.is_none() {
                debug!("Encoded frame buffers channel closed, terminating.");
                break;
            }

            let mut encoded_frame_buffer = encoded_frame_buffer.unwrap();

            let reception_start_time = Instant::now();
            let receive_result = frame_receiver
                .receive_encoded_frame(&mut encoded_frame_buffer)
                .await;
            let reception_time = reception_start_time.elapsed().as_millis();

            let reception_delay  = if receive_result.is_ok() {
                let received_frame= receive_result.as_ref().unwrap();
                received_frame.reception_delay
            } else {
                0
            };

            let received_frame = receive_result.unwrap();

            let mut frame_stats = ReceivedFrameStats::default();
            frame_stats.capture_timestamp = received_frame.capture_timestamp;
            frame_stats.reception_time = reception_time;
            frame_stats.reception_delay = reception_delay;
            frame_stats.receiver_idle_time = encoded_frame_buffer_wait_time;

            let send_result = receive_result_sender.send(ReceiveResult {
                received_frame,
                encoded_frame_buffer,
                frame_stats,
            });

            if let Err(e) = send_result {
                warn!("Capture result send error: {}", e);
                break;
            };
        }
    })
}
