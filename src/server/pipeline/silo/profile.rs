use std::time::{Duration, Instant};

use bytes::BytesMut;
use log::debug;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{Receiver, UnboundedReceiver},
    },
    task::JoinHandle,
};

use crate::{
    common::{feedback::FeedbackMessage, helpers::silo::channel_pull},
    server::profiling::ServerProfiler,
};

use super::transfer::TransferResult;

pub fn launch_profile_thread(
    mut profilers: Vec<Box<dyn ServerProfiler + Send>>,
    mut transfer_result_receiver: UnboundedReceiver<TransferResult>,
    feedback_sender: Sender<FeedbackMessage>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let (transfer_result, total_time) =
                pull_transfer_result(&mut transfer_result_receiver).await;

            let mut frame_data = transfer_result.frame_data;
            frame_data.set_local("total_time", total_time);

            let profilers_count = profilers.len();
            for i in 0..profilers_count {
                let profiler = profilers.get_mut(i).unwrap();

                profiler.log_frame(frame_data.clone_without_buffers());

                broadcast_feedbacks(profiler, &feedback_sender).await;
            }
        }
    })
}

async fn broadcast_feedbacks(
    profiler: &mut Box<dyn ServerProfiler + Send>,
    feedback_sender: &Sender<FeedbackMessage>,
) {
    while let Some(message) = profiler.pull_feedback().await {
        feedback_sender
            .send(message)
            .expect("Unable to broadcast a feedback message");
    }
}

async fn pull_transfer_result(
    transfer_result_receiver: &mut UnboundedReceiver<TransferResult>,
) -> (TransferResult, u128) {
    let (transfer_result, total_time) = channel_pull(transfer_result_receiver)
        .await
        .expect("Transfer result channel closed, terminating.");
    (transfer_result, total_time)
}
