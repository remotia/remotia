use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameHeader {
    pub frame_size: usize
} 

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameBody {
    pub frame_pixels: Vec<u8>
}