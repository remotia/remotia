use std::time::{Duration, Instant};

use bytes::BytesMut;
use log::debug;
use tokio::{sync::mpsc::{Receiver, UnboundedReceiver}, task::JoinHandle};

use crate::server::{profiling::{TransmissionRoundStats, TransmittedFrameStats}, utils::profilation::setup_round_stats};

use super::transfer::TransferResult;

/*pub struct ProfileResult {
    pub last_frame_transmission_time: i64
}*/

pub fn launch_profile_thread(
    csv_profiling: bool, 
    console_profiling: bool,
    mut transfer_result_receiver: UnboundedReceiver<TransferResult>,
    round_duration: Duration
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut round_stats = setup_round_stats(csv_profiling, console_profiling).unwrap();

        loop {
            let result_receive_start_time = Instant::now();
            let transfer_result = transfer_result_receiver.recv().await;
            let total_time = result_receive_start_time.elapsed().as_millis(); 

            if transfer_result.is_none() {
                debug!("Transfer channel has been closed, terminating.");
                break;
            }
            let transfer_result = transfer_result.unwrap();

            let mut frame_stats = transfer_result.frame_stats;
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