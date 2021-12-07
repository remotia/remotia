use std::time::{Duration, Instant};

use log::debug;
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};

use crate::{client::utils::profilation::setup_round_stats, common::helpers::silo::channel_pull};

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
            let (render_result, total_time) =
                channel_pull(&mut render_result_receiver)
                    .await
                    .expect("Render channel has been closed, terminating");

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