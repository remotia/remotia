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

use crate::{common::{feedback::FeedbackMessage, helpers::silo::channel_pull}, server::{capture::FrameCapturer}, types::FrameData};

pub struct CaptureResult {
    pub capture_time: Instant,
    pub frame_data: FrameData,
}

pub fn launch_capture_thread(
    frames_capture_rate: u32,
    mut raw_frame_buffers_receiver: UnboundedReceiver<BytesMut>,
    capture_result_sender: UnboundedSender<CaptureResult>,
    mut frame_capturer: Box<dyn FrameCapturer + Send>,
    mut feedback_receiver: broadcast::Receiver<FeedbackMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let tick_duration = (1000.0 / frames_capture_rate as f64) as u64;
        let mut interval = tokio::time::interval(Duration::from_millis(tick_duration));

        loop {
            let spin_start_time = Instant::now();
            interval.tick().await;
            let spin_time = spin_start_time.elapsed().as_millis();

            pull_feedback(&mut feedback_receiver, &mut frame_capturer);

            let (raw_frame_buffer, raw_frame_buffer_wait_time) =
                pull_raw_buffer(&mut raw_frame_buffers_receiver).await;

            let mut frame_data = FrameData::default();

            frame_data.insert_writable_buffer("raw_frame_buffer", raw_frame_buffer);

            let (capture_start_time, capture_timestamp) =
                capture(&mut frame_capturer, &mut frame_data);

            frame_data.set("capture_timestamp", capture_timestamp);
            frame_data.set("capture_time", capture_start_time.elapsed().as_millis());
            frame_data.set("spin_time", spin_time);
            frame_data.set("capturer_raw_frame_buffer_wait_time", raw_frame_buffer_wait_time);

            if let ControlFlow::Break(_) = push_result(
                &capture_result_sender,
                CaptureResult {
                    capture_time: capture_start_time,
                    frame_data,
                },
            ) {
                break;
            }
        }
    })
}

fn push_result(
    capture_result_sender: &UnboundedSender<CaptureResult>,
    result: CaptureResult,
) -> ControlFlow<()> {
    let send_result = capture_result_sender.send(result);
    if let Err(e) = send_result {
        warn!("Capture result send error: {}", e);
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

fn capture(
    frame_capturer: &mut Box<dyn FrameCapturer + Send>,
    frame_data: &mut FrameData,
) -> (Instant, u128) {
    debug!("Capturing frame...");
    let capture_start_time = Instant::now();
    let capture_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    frame_capturer.capture(frame_data);
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
