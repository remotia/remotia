use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use log::{debug, warn};
use object_pool::{Pool, Reusable};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::server::profiling::TransmittedFrameStats;
use crate::server::send::FrameSender;

use super::encode::EncodeResult;

pub struct TransferResult {
    // pub transmitted_frame_buffer: Reusable<'static, BytesMut>,
    pub frame_stats: TransmittedFrameStats,
}

pub fn launch_transfer_thread(
    mut frame_sender: Box<dyn FrameSender + Send>,
    encoded_frame_buffers_pool: Arc<Pool<BytesMut>>,
    mut encode_result_receiver: Receiver<EncodeResult>,
    transfer_result_sender: Sender<TransferResult>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            debug!("Transferring...");

            let transfer_start_time = Instant::now();

            let encode_result = encode_result_receiver.blocking_recv();
            if encode_result.is_none() {
                debug!("Encode channel has been closed, terminating...");
                break;
            }
            let encode_result = encode_result.unwrap();
            let encoded_frame_buffer = encode_result.encoded_frame_buffer;
            let mut frame_stats = encode_result.frame_stats;

            frame_sender
                .send_frame(&encoded_frame_buffer[..frame_stats.encoded_size])
                .await;

            encoded_frame_buffers_pool.attach(encoded_frame_buffer);

            frame_stats.transfer_time = transfer_start_time.elapsed().as_millis();

            let send_result = transfer_result_sender.send(TransferResult {
                frame_stats,
            }).await;

            if let Err(_) = send_result {
                warn!("Transfer result sender error");
                break;
            };
        }
    })
}
