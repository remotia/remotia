use std::time::{Duration, Instant};

use log::debug;
use tokio::{
    sync::{broadcast, mpsc::UnboundedReceiver},
    task::JoinHandle,
};

use crate::{
    client::utils::profilation::setup_round_stats,
    common::{feedback::FeedbackMessage, helpers::silo::channel_pull},
};

use super::render::RenderResult;
use crate::client::profiling::ClientProfiler;

pub fn launch_profile_thread(
    mut profiler: Box<dyn ClientProfiler + Send>,
    mut render_result_receiver: UnboundedReceiver<RenderResult>,
    csv_profiling: bool,
    console_profiling: bool,
    feedback_sender: broadcast::Sender<FeedbackMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let round_duration = Duration::from_secs(1);
        let mut round_stats = setup_round_stats(csv_profiling, console_profiling).unwrap();

        loop {
            let (render_result, total_time) = channel_pull(&mut render_result_receiver)
                .await
                .expect("Render channel has been closed, terminating");

            let mut frame_stats = render_result.frame_stats;
            frame_stats.total_time = total_time;

            send_stats_to_profiler(&mut profiler, frame_stats, &feedback_sender).await;
            send_stats_to_round_stats(&mut round_stats, frame_stats, round_duration);
        }
    })
}

fn send_stats_to_round_stats(round_stats: &mut crate::client::profiling::ReceptionRoundStats, frame_stats: crate::client::profiling::ReceivedFrameStats, round_duration: Duration) {
    round_stats.profile_frame(frame_stats);
    let current_round_duration = round_stats.start_time.elapsed();
    if current_round_duration.gt(&round_duration) {
        round_stats.log();
        round_stats.reset();
    }
}

async fn send_stats_to_profiler(
    profiler: &mut Box<dyn ClientProfiler + Send>,
    frame_stats: crate::client::profiling::ReceivedFrameStats,
    feedback_sender: &broadcast::Sender<FeedbackMessage>,
) {
    let feedback_message = profiler.profile_frame(frame_stats).await;
    if let Some(message) = feedback_message {
        feedback_sender
            .send(message)
            .expect("Unable to broadcast a feedback message");
    }
}
