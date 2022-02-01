use std::{
    ops::ControlFlow,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bytes::BytesMut;
use chrono::Utc;
use log::{debug, info, warn};
use tokio::{
    sync::{
        broadcast,
        mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender},
    },
    task::JoinHandle,
};

use crate::{
    common::{feedback::FeedbackMessage, helpers::silo::channel_pull},
    server::{
        capture::FrameCapturer, profiling::TransmittedFrameStats,
        utils::encoding::packed_bgra_to_packed_bgr,
    },
};

pub struct CaptureResult {
    pub capture_timestamp: u128,
    pub capture_time: Instant,

    pub raw_frame_buffer: BytesMut,
    pub frame_stats: TransmittedFrameStats,
}

pub fn launch_capture_thread(
    spin_time: i64,
    mut raw_frame_buffers_receiver: UnboundedReceiver<BytesMut>,
    capture_result_sender: UnboundedSender<CaptureResult>,
    mut frame_capturer: Box<dyn FrameCapturer + Send>,
    mut feedback_receiver: broadcast::Receiver<FeedbackMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_frame_capture_time: i64 = 0;

        loop {
            pull_feedback(&mut feedback_receiver, &mut frame_capturer);

            spin(spin_time, last_frame_capture_time).await;

            let (mut raw_frame_buffer, raw_frame_buffer_wait_time) =
                pull_raw_buffer(&mut raw_frame_buffers_receiver).await;

            let (capture_start_time, capture_timestamp) =
                capture(&mut frame_capturer, &mut raw_frame_buffer);

            let frame_stats = initialize_frame_stats(
                &mut last_frame_capture_time,
                capture_start_time,
                capture_timestamp,
                raw_frame_buffer_wait_time,
            );

            if let ControlFlow::Break(_) = push_result(
                &capture_result_sender,
                capture_timestamp,
                capture_start_time,
                raw_frame_buffer,
                frame_stats,
            ) {
                break;
            }
        }
    })
}

fn push_result(
    capture_result_sender: &UnboundedSender<CaptureResult>,
    capture_timestamp: u128,
    capture_start_time: Instant,
    raw_frame_buffer: BytesMut,
    frame_stats: TransmittedFrameStats,
) -> ControlFlow<()> {
    let send_result = capture_result_sender.send(CaptureResult {
        capture_timestamp,
        capture_time: capture_start_time,
        raw_frame_buffer,
        frame_stats,
    });
    if let Err(e) = send_result {
        warn!("Capture result send error: {}", e);
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

fn initialize_frame_stats(
    last_frame_capture_time: &mut i64,
    capture_start_time: Instant,
    capture_timestamp: u128,
    raw_frame_buffer_wait_time: u128,
) -> TransmittedFrameStats {
    let mut frame_stats = TransmittedFrameStats::default();
    *last_frame_capture_time = capture_start_time.elapsed().as_millis() as i64;

    frame_stats.capture_timestamp = capture_timestamp;
    frame_stats.capture_time = *last_frame_capture_time as u128;
    frame_stats.capturer_idle_time = raw_frame_buffer_wait_time;
    frame_stats
}

fn capture(
    frame_capturer: &mut Box<dyn FrameCapturer + Send>,
    raw_frame_buffer: &mut BytesMut,
) -> (Instant, u128) {
    debug!("Capturing frame...");
    let capture_start_time = Instant::now();
    let capture_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    frame_capturer.capture(raw_frame_buffer).unwrap();
    debug!("Frame captured");
    (capture_start_time, capture_timestamp)
}

async fn pull_raw_buffer(
    raw_frame_buffers_receiver: &mut UnboundedReceiver<BytesMut>,
) -> (BytesMut, u128) {
    let (raw_frame_buffer, raw_frame_buffer_wait_time) = channel_pull(raw_frame_buffers_receiver)
        .await
        .expect("Raw frame buffers channel closed, terminating.");
    (raw_frame_buffer, raw_frame_buffer_wait_time)
}

async fn spin(spin_time: i64, last_frame_capture_time: i64) {
    let sleep_time = std::cmp::max(0, spin_time - last_frame_capture_time) as u64;
    tokio::time::sleep(Duration::from_millis(sleep_time)).await;
}

fn pull_feedback(
    feedback_receiver: &mut broadcast::Receiver<FeedbackMessage>,
    frame_capturer: &mut Box<dyn FrameCapturer + Send>,
) {
    match feedback_receiver.try_recv() {
        Ok(message) => {
            frame_capturer.handle_feedback(message);
        }
        Err(_) => {}
    };
}
