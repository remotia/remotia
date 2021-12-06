use std::time::Instant;

use bytes::BytesMut;
use log::{debug, warn};
use pixels::Pixels;
use tokio::{sync::mpsc::{UnboundedReceiver, UnboundedSender}, task::JoinHandle};

use crate::client::{decode::Decoder, error::ClientError, profiling::ReceivedFrameStats, utils::decoding::packed_bgr_to_packed_rgba};

use super::decode::DecodeResult;

pub struct RenderResult {
    pub frame_stats: ReceivedFrameStats
}

pub fn launch_render_thread(
    mut pixels: Pixels,
    raw_frame_buffers_sender: UnboundedSender<BytesMut>,
    mut decode_result_receiver: UnboundedReceiver<DecodeResult>,
    render_result_sender: UnboundedSender<RenderResult>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let decode_result_wait_start_time = Instant::now();
            let decode_result = decode_result_receiver.recv().await;
            let decode_result_wait_time = decode_result_wait_start_time.elapsed().as_millis();
            if decode_result.is_none() {
                debug!("Decode channel has been closed, terminating");
                break;
            }
            let decode_result = decode_result.unwrap();

            let raw_frame_buffer =  decode_result.raw_frame_buffer;

            let rendering_start_time = Instant::now();
            packed_bgr_to_packed_rgba(&raw_frame_buffer, pixels.get_frame());
            pixels.render().unwrap();
            let rendering_time = rendering_start_time.elapsed().as_millis();

            let buffer_return_result = raw_frame_buffers_sender.send(raw_frame_buffer);
            if let Err(e) = buffer_return_result {
                warn!("Raw frame buffer return error: {}", e);
                break;
            };

            let mut frame_stats = decode_result.frame_stats;
            frame_stats.rendering_time = rendering_time;
            frame_stats.renderer_idle_time = decode_result_wait_time;

            let send_result = render_result_sender.send(RenderResult {
                frame_stats
            });

            if let Err(e) = send_result {
                warn!("Render result send error: {}", e);
                break;
            };
        } 
    })
}

fn _recalculate_fps(current_fps: f64, target_fps: f64, frame_error: Option<&ClientError>) -> f64 {
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