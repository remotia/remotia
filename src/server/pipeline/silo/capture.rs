use std::{sync::{Arc, Mutex}, thread, time::{Duration, Instant}};

use bytes::BytesMut;
use log::{debug, info, warn};
use object_pool::{Pool, Reusable};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task::JoinHandle,
};

use crate::server::{
    capture::FrameCapturer, profiling::TransmittedFrameStats,
    utils::encoding::packed_bgra_to_packed_bgr,
};

pub struct CaptureResult {
    pub raw_frame_buffer: BytesMut,
    pub frame_stats: TransmittedFrameStats,
}

pub fn launch_capture_thread(
    spin_time: i64,
    raw_frame_buffers_pool: Arc<Pool<BytesMut>>,
    capture_result_sender: Sender<CaptureResult>,
    mut frame_capturer: Box<dyn FrameCapturer + Send>
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_frame_capture_time;

        loop {
            debug!("Capturing frame...");

            thread::sleep(Duration::from_millis(std::cmp::max(0, spin_time) as u64));

            let capture_start_time = Instant::now();

            let result = frame_capturer.capture();

            debug!("Frame captured");

            let packed_bgra_frame_buffer = result.unwrap();

            let raw_frame_buffer = raw_frame_buffers_pool.try_pull();
            let raw_frame_buffer = raw_frame_buffer.unwrap();
            let (_, mut raw_frame_buffer) = raw_frame_buffer.detach();

            packed_bgra_to_packed_bgr(&packed_bgra_frame_buffer, &mut raw_frame_buffer);

            let mut frame_stats = TransmittedFrameStats::default();

            last_frame_capture_time = capture_start_time.elapsed().as_millis();
            frame_stats.capture_time = last_frame_capture_time;

            /*let send_result = capture_result_sender.send(CaptureResult {
                raw_frame_buffer,
                frame_stats,
            }).await;

            if let Err(e) = send_result {
                warn!("Capture result send error: {}", e);
                break;
            };*/
        }
    })
}
