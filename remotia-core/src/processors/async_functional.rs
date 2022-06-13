use std::pin::Pin;

use async_trait::async_trait;
use futures::Future;

use crate::{traits::FrameProcessor, types::FrameData};

pub type PinnedFrameData = Pin<Box<dyn Future<Output = Option<FrameData>> + Send>>;
type AsyncProcessorFn = fn(FrameData) -> PinnedFrameData;

pub struct AsyncFunction {
    function: AsyncProcessorFn
}

impl AsyncFunction {
    pub fn new(function: AsyncProcessorFn) -> Self {
        Self {
            function,
        }
    }
}

#[async_trait]
impl FrameProcessor for AsyncFunction {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        (self.function)(frame_data).await
    }
}

#[macro_export]
macro_rules! async_func {
    (async move $body:block) => {
        Box::pin(async move { $body })
    };
}
