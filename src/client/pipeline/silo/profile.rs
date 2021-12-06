use std::time::{Duration, Instant};

use log::debug;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};

use crate::client::utils::profilation::setup_round_stats;

use super::render::RenderResult;


pub fn launch_profile_thread(
    mut render_result_receiver: UnboundedReceiver<RenderResult>,
    csv_profiling: bool,
    console_profiling: bool
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let round_duration = Duration::from_secs(1);
        let mut round_stats =
            setup_round_stats(csv_profiling, console_profiling).unwrap();

        loop {
            let result_receive_start_time = Instant::now();
            let render_result = render_result_receiver.recv().await;
            let total_time = result_receive_start_time.elapsed().as_millis();
            if render_result.is_none() {
                debug!("Render channel has been closed, terminating");
                break;
            }
            let render_result = render_result.unwrap();

            let mut frame_stats = render_result.frame_stats;
            frame_stats.total_time = total_time;

            round_stats.profile_frame(frame_stats);

            let current_round_duration = round_stats.start_time.elapsed();

            if current_round_duration.gt(&round_duration) {
                round_stats.log();
                round_stats.reset();
            }
        }
    })
}