use std::{
    ops::ControlFlow,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bytes::BytesMut;
use log::{debug, info, warn};
use pixels::Pixels;
use tokio::{
    sync::{
        broadcast,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    task::JoinHandle,
};

use crate::{
    client::{
        decode::Decoder, error::ClientError, profiling::ReceivedFrameStats, render::Renderer,
    },
    common::{feedback::FeedbackMessage, helpers::silo::channel_pull},
};

use super::decode::DecodeResult;

pub struct RenderResult {
    pub frame_stats: ReceivedFrameStats,
}

pub fn launch_render_thread(
    mut renderer: Box<dyn Renderer + Send>,
    target_fps: u32,
    maximum_pre_render_frame_delay: u128,
    raw_frame_buffers_sender: UnboundedSender<BytesMut>,
    mut decode_result_receiver: UnboundedReceiver<DecodeResult>,
    render_result_sender: UnboundedSender<RenderResult>,
    mut feedback_receiver: broadcast::Receiver<FeedbackMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let target_fps = target_fps as f64;
        let mut fps: f64 = recalculate_fps(0.0, target_fps, None);
        let mut last_spin_time: u64 = 0;

        loop {
            pull_feedback_messages(&mut feedback_receiver, &mut renderer);

            let frame_dispatch_start_time = Instant::now();

            let (decode_result, decode_result_wait_time, mut frame_stats) =
                pull_decode_results(&mut decode_result_receiver).await;

            if decode_result.raw_frame_buffer.is_some() {
                let raw_frame_buffer = decode_result.raw_frame_buffer.unwrap();

                let pre_render_frame_delay = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    - frame_stats.capture_timestamp;

                if pre_render_frame_delay > maximum_pre_render_frame_delay {
                    frame_stats.error = Some(ClientError::StaleFrame);
                } else {
                    update_frame_buffer(
                        &raw_frame_buffer,
                        &mut frame_stats,
                        &mut renderer,
                        decode_result_wait_time,
                        last_spin_time,
                    );
                }

                if let ControlFlow::Break(_) =
                    return_buffer(&raw_frame_buffers_sender, raw_frame_buffer)
                {
                    break;
                }
            }

            fps = recalculate_fps(fps, target_fps, frame_stats.error.as_ref());

            let frame_dispatch_time =
                calculate_frame_dispatch_time(frame_stats, frame_dispatch_start_time);

            if let ControlFlow::Break(_) = push_result(&render_result_sender, frame_stats) {
                break;
            }

            spin(fps, frame_dispatch_time, &mut last_spin_time).await;
        }
    })
}

fn return_buffer(
    raw_frame_buffers_sender: &UnboundedSender<BytesMut>,
    raw_frame_buffer: BytesMut,
) -> ControlFlow<()> {
    debug!("Returning the raw frame buffer back...");
    let buffer_return_result = raw_frame_buffers_sender.send(raw_frame_buffer);
    if let Err(e) = buffer_return_result {
        warn!("Raw frame buffer return error: {}", e);
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

async fn spin(fps: f64, frame_dispatch_time: i64, last_spin_time: &mut u64) {
    let spin_time = (1000 / std::cmp::max(fps as i64, 1)) - frame_dispatch_time;
    *last_spin_time = std::cmp::max(0, spin_time) as u64;
    tokio::time::sleep(Duration::from_millis(*last_spin_time)).await;
}

fn calculate_frame_dispatch_time(
    frame_stats: ReceivedFrameStats,
    frame_dispatch_start_time: Instant,
) -> i64 {
    let frame_dispatch_time = (frame_stats.reception_time
        + frame_stats.decoding_time
        + frame_dispatch_start_time.elapsed().as_millis()) as i64;
    frame_dispatch_time
}

fn push_result(
    render_result_sender: &UnboundedSender<RenderResult>,
    frame_stats: ReceivedFrameStats,
) -> ControlFlow<()> {
    let send_result = render_result_sender.send(RenderResult { frame_stats });
    if let Err(e) = send_result {
        warn!("Render result send error: {}", e);
        return ControlFlow::Break(());
    };
    ControlFlow::Continue(())
}

fn update_frame_buffer(
    raw_frame_buffer: &BytesMut,
    frame_stats: &mut ReceivedFrameStats,
    renderer: &mut Box<dyn Renderer + Send>,
    decode_result_wait_time: u128,
    last_spin_time: u64,
) {
    if frame_stats.error.is_none() {
        debug!("Rendering frame with stats: {:?}", frame_stats);

        let rendering_start_time = Instant::now();
        renderer.render(&raw_frame_buffer);
        let rendering_time = rendering_start_time.elapsed().as_millis();

        let frame_delay = {
            let capture_timestamp = frame_stats.capture_timestamp;
            let frame_delay = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                - capture_timestamp;

            frame_delay
        };

        frame_stats.rendering_time = rendering_time;
        frame_stats.frame_delay = frame_delay;
        frame_stats.renderer_idle_time = decode_result_wait_time;
        frame_stats.spin_time = last_spin_time;
    }
}

async fn pull_decode_results(
    decode_result_receiver: &mut UnboundedReceiver<DecodeResult>,
) -> (DecodeResult, u128, ReceivedFrameStats) {
    debug!("Waiting for the decode result...");
    let (decode_result, decode_result_wait_time) = channel_pull(decode_result_receiver)
        .await
        .expect("Decode channel has been closed, terminating");
    let frame_stats = decode_result.frame_stats;
    (decode_result, decode_result_wait_time, frame_stats)
}

fn pull_feedback_messages(
    feedback_receiver: &mut broadcast::Receiver<FeedbackMessage>,
    renderer: &mut Box<dyn Renderer + Send>,
) {
    match feedback_receiver.try_recv() {
        Ok(message) => {
            renderer.handle_feedback(message);
        }
        Err(_) => {}
    };
}

fn recalculate_fps(current_fps: f64, target_fps: f64, frame_error: Option<&ClientError>) -> f64 {
    if let Some(error) = frame_error {
        match error {
            ClientError::Timeout => current_fps * 0.6,
            _ => current_fps,
        }
    } else {
        let fps_increment = (target_fps - current_fps) / 2.0;
        let next_round_fps = current_fps + fps_increment;
        next_round_fps
    }
}
