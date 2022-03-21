use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct RemVSPFrameHeader {
    pub frame_fragments_count: u16,
    pub fragment_size: u16,
    pub capture_timestamp: u128
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemVSPFrameFragment {
    pub frame_header: RemVSPFrameHeader,
    pub fragment_id: u16,
    pub data: Vec<u8>
}