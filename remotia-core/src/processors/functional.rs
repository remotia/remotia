use async_trait::async_trait;

use crate::{traits::FrameProcessor, types::FrameData};

type ProcessorFn = fn(FrameData) -> Option<FrameData>;

pub struct Function {
    function: ProcessorFn
}

impl Function {
    pub fn new(function: ProcessorFn) -> Self {
        Self {
            function,
        }
    }
}

#[async_trait]
impl FrameProcessor for Function {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        (self.function)(frame_data)
    }
}