use std::{collections::HashMap, fmt::Debug, net::SocketAddr, sync::Arc, time::Duration};

use async_trait::async_trait;

use itertools::Itertools;
use log::{debug, info};
use socket2::{Domain, Socket, Type};
use tokio::{
    net::UdpSocket,
    sync::{Mutex, MutexGuard},
    time::Instant,
};

use crate::{client::receive::ReceivedFrame, common::network::remvsp::RemVSPFrameFragment};

use super::reconstruct::FrameReconstructionState;

#[derive(Default, Debug)]
pub struct RemVSPReceptionState {
    pub(crate) last_reconstructed_frame: usize,
    pub(crate) frames_in_reception: HashMap<usize, FrameReconstructionState>,
}

impl RemVSPReceptionState {
    pub fn register_frame_fragment(&mut self, fragment: RemVSPFrameFragment) {
        let frame_id = fragment.frame_header.frame_id;

        let frame_reconstruction_state = {
            let frames_in_reception = &mut self.frames_in_reception;

            let frame_reconstruction_state = frames_in_reception.get_mut(&frame_id);

            if frame_reconstruction_state.is_some() {
                debug!(
                    "Frame {} has been partially received, already, updating the reconstruction state", frame_id
                );
                frame_reconstruction_state.unwrap()
            } else {
                frames_in_reception.insert(frame_id, FrameReconstructionState::default());
                frames_in_reception.get_mut(&frame_id).unwrap()
            }
        };

        if frame_reconstruction_state.has_received_fragment(&fragment) {
            debug!("Duplicate fragment, dropping");
        } else {
            frame_reconstruction_state.register_fragment(fragment);
        }
    }

    fn is_frame_stale(&self, frame_id: usize) -> bool {
        frame_id <= self.last_reconstructed_frame
    }

    fn drop_frame_data(&mut self, frame_id: usize) {
        self.frames_in_reception.remove(&frame_id);
    }

    fn reconstruct_frame(&mut self, frame_id: usize, output_buffer: &mut [u8]) -> ReceivedFrame {
        let frame = self
            .frames_in_reception
            .remove(&frame_id)
            .expect("Retrieving a non-existing frame");

        let reception_delay = frame.first_reception.elapsed().as_millis();

        let frame_header = frame.frame_header.unwrap();
        let capture_timestamp = frame_header.capture_timestamp;

        let buffer_size = frame.reconstruct(output_buffer);

        self.last_reconstructed_frame = frame_id;

        ReceivedFrame {
            buffer_size,
            capture_timestamp,
            reception_delay,
        }
    }

    pub fn pull_frame(&mut self, encoded_frame_buffer: &mut [u8]) -> Option<ReceivedFrame> {
        debug!("Frames reception state: {:#?}", self);

        let mut pulled_frame: Option<ReceivedFrame> = None;

        let mut stale_frames: Vec<usize> = Vec::new();

        info!("Frames to check: {}", self.frames_in_reception.len());

        for frame_id in self.frames_in_reception.keys().sorted() {
            let frame = self.frames_in_reception.get(frame_id).unwrap();

            if frame.is_complete() {
                let frame_id = *frame_id;

                debug!(
                    "Frame #{} has been received completely. Last received frame: {}",
                    frame_id, self.last_reconstructed_frame
                );

                if self.is_frame_stale(frame_id) {
                    stale_frames.push(frame_id);
                    continue;
                }

                let received_frame = self.reconstruct_frame(frame_id, encoded_frame_buffer);

                pulled_frame = Some(received_frame);
                break;
            }
        }

        for frame_id in stale_frames {
            info!("Frame {} is stale, dropping...", frame_id);
            self.drop_frame_data(frame_id);
        }

        pulled_frame
    }
}