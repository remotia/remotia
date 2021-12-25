use std::collections::HashMap;

use std::fmt::Debug;

use tokio::time::Instant;

use crate::common::network::remvsp::{RemVSPFrameFragment, RemVSPFrameHeader};

const DELAYABLE_FRAME_THRESHOLD: u128 = 100;

pub struct FrameReconstructionState {
    pub(crate) frame_header: Option<RemVSPFrameHeader>,
    pub(crate) first_reception: Instant,
    pub(crate) received_fragments: HashMap<u16, Vec<u8>>,
}

impl Default for FrameReconstructionState {
    fn default() -> Self {
        Self {
            frame_header: Default::default(),
            first_reception: Instant::now(),
            received_fragments: Default::default(),
        }
    }
}

impl Debug for FrameReconstructionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let received_fragments_resume = format!(
            "{}/{}",
            self.received_fragments.len(),
            self.frame_header.unwrap().frame_fragments_count
        );

        f.debug_struct("FrameReconstructionState")
            .field("frame_header", &self.frame_header)
            .field("received_fragments", &received_fragments_resume)
            .finish()
    }
}

impl FrameReconstructionState {
    pub fn has_received_fragment(&self, fragment: &RemVSPFrameFragment) -> bool {
        self.received_fragments.contains_key(&fragment.fragment_id)
    }

    pub fn register_fragment(&mut self, fragment: RemVSPFrameFragment) {
        if self.frame_header.is_none() {
            self.frame_header = Some(fragment.frame_header);
        }

        self.received_fragments
            .insert(fragment.fragment_id, fragment.data);
    }

    pub fn is_delayable(&self) -> bool {
        self.first_reception.elapsed().as_millis() < DELAYABLE_FRAME_THRESHOLD
    }

    pub fn is_complete(&self) -> bool {
        if self.frame_header.is_some() {
            let received_fragments = self.received_fragments.len() as u16;
            let frame_fragments = self.frame_header.unwrap().frame_fragments_count;

            return received_fragments == frame_fragments;
        }

        return false;
    }

    pub fn reconstruct(self, buffer: &mut [u8]) -> usize {
        let mut written_bytes = 0;

        let frame_header = self
            .frame_header
            .expect("Reconstructing without a frame header");

        let fragment_size = frame_header.fragment_size as usize;

        for (fragment_id, data) in self.received_fragments.into_iter() {
            let current_fragment_data_size = data.len();
            let fragment_id = fragment_id as usize;
            let fragment_offset = (fragment_id * fragment_size) as usize;

            let fragment_buffer =
                &mut buffer[fragment_offset..fragment_offset + current_fragment_data_size];

            fragment_buffer.copy_from_slice(&data);

            written_bytes += current_fragment_data_size;
        }

        written_bytes
    }
}