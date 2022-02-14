use std::{
    ops::ControlFlow,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bytes::BytesMut;
use log::{debug, info, warn};
use tokio::{
    sync::{
        broadcast,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    task::JoinHandle,
};

use crate::{
    client::{
        decode::Decoder, profiling::ReceivedFrameStats, render::Renderer,
    },
    common::{feedback::FeedbackMessage, helpers::silo::channel_pull},
    error::DropReason
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

        let tick_duration = (1000.0 / target_fps) as u64;
        let mut interval = tokio::time::interval(Duration::from_millis(tick_duration));

        loop {
            pull_feedback_messages(&mut feedback_receiver, &mut renderer);

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
                    frame_stats.error = Some(DropReason::StaleFrame);
                } else {
                    let spin_start_time = Instant::now();
                    interval.tick().await;
                    let spin_time = spin_start_time.elapsed().as_millis() as u64;

                    if frame_stats.error.is_none() {
                        let (rendering_time, frame_delay) = update_frame_buffer(
                            &raw_frame_buffer,
                            frame_stats.capture_timestamp,
                            &mut renderer,
                        );

                        frame_stats.rendering_time = rendering_time;
                        frame_stats.frame_delay = frame_delay;
                        frame_stats.spin_time = spin_time;
                    }

                    frame_stats.renderer_idle_time = decode_result_wait_time;

                    // last_render_instant = Instant::now();
                }

                if let ControlFlow::Break(_) =
                    return_buffer(&raw_frame_buffers_sender, raw_frame_buffer)
                {
                    break;
                }
            }

            if let ControlFlow::Break(_) = push_result(&render_result_sender, frame_stats) {
                break;
            }
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
    capture_timestamp: u128,
    renderer: &mut Box<dyn Renderer + Send>,
) -> (u128, u128) {
    debug!("Updating frame buffer...");

    let rendering_start_time = Instant::now();
    renderer.render(&raw_frame_buffer);
    let rendering_time = rendering_start_time.elapsed().as_millis();

    let frame_delay = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        - capture_timestamp;

    (rendering_time, frame_delay)
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
