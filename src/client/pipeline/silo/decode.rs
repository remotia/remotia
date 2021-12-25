use std::time::Instant;

use bytes::BytesMut;
use log::{debug, warn};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::{
    client::{decode::Decoder, error::ClientError, profiling::ReceivedFrameStats},
    common::helpers::silo::channel_pull,
};

use super::receive::ReceiveResult;

pub struct DecodeResult {
    pub raw_frame_buffer: Option<BytesMut>,

    pub frame_stats: ReceivedFrameStats,
}

pub fn launch_decode_thread(
    mut decoder: Box<dyn Decoder + Send>,
    mut raw_frame_buffers_receiver: UnboundedReceiver<BytesMut>,
    encoded_frame_buffers_sender: UnboundedSender<BytesMut>,
    mut receive_result_receiver: UnboundedReceiver<ReceiveResult>,
    decode_result_sender: UnboundedSender<DecodeResult>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            debug!("Waiting for receive result...");
            let (receive_result, receive_result_wait_time) =
                channel_pull(&mut receive_result_receiver)
                    .await
                    .expect("Receive channel has been closed, terminating");

            let mut frame_stats = receive_result.frame_stats;

            let encoded_frame_buffer = receive_result.encoded_frame_buffer;

            let raw_frame_buffer = if frame_stats.error.is_none() {
                debug!("Pulling raw frame buffer...");

                let (mut raw_frame_buffer, raw_frame_buffer_wait_time) =
                    channel_pull(&mut raw_frame_buffers_receiver)
                        .await
                        .expect("Raw frame buffer channel has been closed, terminating");

                let received_frame = receive_result.received_frame.unwrap();

                debug!("Sending the encoded frame to the decoder...");
                let decoding_start_time = Instant::now();
                let decoder_result = decoder.decode(
                    &encoded_frame_buffer[..received_frame.buffer_size],
                    &mut raw_frame_buffer,
                );
                let decoding_time = decoding_start_time.elapsed().as_millis();

                if decoder_result.is_err() {
                    frame_stats.error = Some(decoder_result.unwrap_err());
                    debug!("Decode error: {:?}", frame_stats.error);
                }

                frame_stats.decoding_time = decoding_time;
                frame_stats.decoder_idle_time =
                    receive_result_wait_time + raw_frame_buffer_wait_time;

                Some(raw_frame_buffer)
            } else {
                debug!("Receive error: {:?}", frame_stats.error);
                None
            };

            let buffer_return_result =
                encoded_frame_buffers_sender.send(encoded_frame_buffer);
            if let Err(e) = buffer_return_result {
                warn!("Encoded frame buffer return error: {}", e);
                break;
            };

            debug!("Sending the decode result...");
            let send_result = decode_result_sender.send(DecodeResult {
                raw_frame_buffer,
                frame_stats,
            });

            if let Err(e) = send_result {
                warn!("Decode result send error: {}", e);
                break;
            };
        }
    })
}
