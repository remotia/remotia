use serde::{Deserialize, Serialize};

pub mod remvsp;

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameBody {
    pub capture_timestamp: u128,
    pub frame_pixels: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameHeader {
    pub capture_timestamp: u128,
    pub fragments_count: usize
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameFragment {
    pub index: usize,
    pub data: Vec<u8>
}
