use async_trait::async_trait;

use crate::{traits::FrameProcessor};

pub struct Function<F> {
    function: fn(F) -> Option<F>
}

impl<F> Function<F> {
    pub fn new(function: fn(F) -> Option<F>) -> Self {
        Self {
            function,
        }
    }
}

#[async_trait]
impl<F: Send> FrameProcessor<F> for Function<F> {
    async fn process(&mut self, frame_data: F) -> Option<F> {
        (self.function)(frame_data)
    }
}