use async_trait::async_trait;
use rand::Rng;

use crate::{types::FrameData, traits::FrameProcessor};

pub struct RandomFrameDropper {
    probability: f32
}

impl RandomFrameDropper {
    pub fn new(probability: f32) -> Self {
        Self {
            probability
        }
    }
}

#[async_trait]
impl FrameProcessor for RandomFrameDropper {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        let mut rng = rand::thread_rng();

        if rng.gen::<f32>() <= self.probability {
            None
        } else {
            Some(frame_data)
        }
    }
}