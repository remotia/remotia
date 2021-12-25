use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bytes::BytesMut;
use log::{debug, info, warn};
use pixels::Pixels;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::{
    client::{
        decode::Decoder, error::ClientError, profiling::ReceivedFrameStats,
        utils::decoding::packed_bgr_to_packed_rgba,
    },
    common::helpers::silo::channel_pull,
};

use super::decode::DecodeResult;

pub struct RenderResult {
    pub frame_stats: ReceivedFrameStats,
}

pub fn launch_render_thread(
    target_fps: u32,
    mut pixels: Pixels,
    raw_frame_buffers_sender: UnboundedSender<BytesMut>,
    mut decode_result_receiver: UnboundedReceiver<DecodeResult>,
    render_result_sender: UnboundedSender<RenderResult>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let target_fps = target_fps as f64;
        let mut fps: f64 = recalculate_fps(0.0, target_fps, None);

        let mut last_spin_time: u64 = 0;

        loop {
            let frame_dispatch_start_time = Instant::now();

            debug!("Waiting for the decode result...");
            let (decode_result, decode_result_wait_time) =
                channel_pull(&mut decode_result_receiver)
                    .await
                    .expect("Decode channel has been closed, terminating");

            let mut frame_stats = decode_result.frame_stats;

            if decode_result.raw_frame_buffer.is_some() {
                let raw_frame_buffer = decode_result.raw_frame_buffer.unwrap();

                if frame_stats.error.is_none() {
                    debug!("Rendering frame with stats: {:?}", frame_stats);

                    let rendering_start_time = Instant::now();
                    packed_bgr_to_packed_rgba(&raw_frame_buffer, pixels.get_frame());
                    pixels.render().unwrap();
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

                debug!("Returning the raw frame buffer back...");
                let buffer_return_result = raw_frame_buffers_sender.send(raw_frame_buffer);
                if let Err(e) = buffer_return_result {
                    warn!("Raw frame buffer return error: {}", e);
                    break;
                };
            }

            fps = recalculate_fps(fps, target_fps, frame_stats.error.as_ref());

            let frame_dispatch_time = (frame_stats.reception_time
                + frame_stats.decoding_time
                + frame_dispatch_start_time.elapsed().as_millis())
                as i64;

            let send_result = render_result_sender.send(RenderResult { frame_stats });

            if let Err(e) = send_result {
                warn!("Render result send error: {}", e);
                break;
            };

            let spin_time = (1000 / std::cmp::max(fps as i64, 1)) - frame_dispatch_time;
            last_spin_time = std::cmp::max(0, spin_time) as u64;
            tokio::time::sleep(Duration::from_millis(last_spin_time)).await;
        }
    })
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
