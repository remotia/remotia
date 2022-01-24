use std::{ops::ControlFlow, time::Instant};

use bytes::BytesMut;
use log::{debug, warn};
use tokio::{
    sync::{
        broadcast,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    task::JoinHandle,
};

use crate::{
    client::{decode::Decoder, error::ClientError, profiling::ReceivedFrameStats},
    common::{feedback::FeedbackMessage, helpers::silo::channel_pull},
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
    mut feedback_receiver: broadcast::Receiver<FeedbackMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            pull_feedback(&mut feedback_receiver, &mut decoder);

            debug!("Waiting for receive result...");
            let (receive_result, receive_result_wait_time) =
                pull_result(&mut receive_result_receiver).await;

            let mut frame_stats = receive_result.frame_stats;

            let encoded_frame_buffer = receive_result.encoded_frame_buffer;

            let raw_frame_buffer = if frame_stats.error.is_none() {
                let (mut raw_frame_buffer, raw_frame_buffer_wait_time) =
                    pull_raw_frame_buffer(&mut raw_frame_buffers_receiver).await;

                let received_frame = receive_result.received_frame.unwrap();

                let decoding_time = decode(
                    &mut decoder,
                    &encoded_frame_buffer,
                    received_frame,
                    &mut raw_frame_buffer,
                    &mut frame_stats,
                );

                update_decoding_stats(
                    &mut frame_stats,
                    decoding_time,
                    receive_result_wait_time,
                    raw_frame_buffer_wait_time,
                );

                Some(raw_frame_buffer)
            } else {
                debug!("Receive error: {:?}", frame_stats.error);
                None
            };

            if let ControlFlow::Break(_) =
                return_buffer(&encoded_frame_buffers_sender, encoded_frame_buffer)
            {
                break;
            }

            if let ControlFlow::Break(_) =
                push_result(&decode_result_sender, raw_frame_buffer, frame_stats)
            {
                break;
            }
        }
    })
}

fn push_result(
    decode_result_sender: &UnboundedSender<DecodeResult>,
    raw_frame_buffer: Option<BytesMut>,
    frame_stats: ReceivedFrameStats,
) -> ControlFlow<()> {
    debug!("Sending the decode result...");
    let send_result = decode_result_sender.send(DecodeResult {
        raw_frame_buffer,
        frame_stats,
    });
    if let Err(e) = send_result {
        warn!("Decode result send error: {}", e);
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

fn return_buffer(
    encoded_frame_buffers_sender: &UnboundedSender<BytesMut>,
    encoded_frame_buffer: BytesMut,
) -> ControlFlow<()> {
    let buffer_return_result = encoded_frame_buffers_sender.send(encoded_frame_buffer);
    if let Err(e) = buffer_return_result {
        warn!("Encoded frame buffer return error: {}", e);
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

fn update_decoding_stats(
    frame_stats: &mut ReceivedFrameStats,
    decoding_time: u128,
    receive_result_wait_time: u128,
    raw_frame_buffer_wait_time: u128,
) {
    frame_stats.decoding_time = decoding_time;
    frame_stats.decoder_idle_time = receive_result_wait_time + raw_frame_buffer_wait_time;
}

fn decode(
    decoder: &mut Box<dyn Decoder + Send>,
    encoded_frame_buffer: &BytesMut,
    received_frame: crate::client::receive::ReceivedFrame,
    raw_frame_buffer: &mut BytesMut,
    frame_stats: &mut ReceivedFrameStats,
) -> u128 {
    debug!("Sending the encoded frame to the decoder...");
    let decoding_start_time = Instant::now();
    let decoder_result = decoder.decode(
        &encoded_frame_buffer[..received_frame.buffer_size],
        raw_frame_buffer,
    );
    let decoding_time = decoding_start_time.elapsed().as_millis();
    if decoder_result.is_err() {
        frame_stats.error = Some(decoder_result.unwrap_err());
        debug!("Decode error: {:?}", frame_stats.error);
    }
    decoding_time
}

async fn pull_raw_frame_buffer(
    raw_frame_buffers_receiver: &mut UnboundedReceiver<BytesMut>,
) -> (BytesMut, u128) {
    debug!("Pulling raw frame buffer...");
    let (raw_frame_buffer, raw_frame_buffer_wait_time) = channel_pull(raw_frame_buffers_receiver)
        .await
        .expect("Raw frame buffer channel has been closed, terminating");
    (raw_frame_buffer, raw_frame_buffer_wait_time)
}

async fn pull_result(
    receive_result_receiver: &mut UnboundedReceiver<ReceiveResult>,
) -> (ReceiveResult, u128) {
    let (receive_result, receive_result_wait_time) = channel_pull(receive_result_receiver)
        .await
        .expect("Receive channel has been closed, terminating");
    (receive_result, receive_result_wait_time)
}

fn pull_feedback(
    feedback_receiver: &mut broadcast::Receiver<FeedbackMessage>,
    decoder: &mut Box<dyn Decoder + Send>,
) {
    match feedback_receiver.try_recv() {
        Ok(message) => {
            decoder.handle_feedback(message);
        }
        Err(_) => {}
    };
}
