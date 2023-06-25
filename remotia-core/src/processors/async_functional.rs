use std::pin::Pin;

use async_trait::async_trait;
use futures::Future;

use crate::{traits::FrameProcessor};

pub type PinnedFrameData<F> = Pin<Box<dyn Future<Output = Option<F>> + Send>>;
type AsyncProcessorFn<F> = fn(F) -> PinnedFrameData<F>;

pub struct AsyncFunction<F> {
    function: AsyncProcessorFn<F>
}

impl<F> AsyncFunction<F> {
    pub fn new(function: AsyncProcessorFn<F>) -> Self {
        Self {
            function,
        }
    }
}

#[async_trait]
impl<F> FrameProcessor<F> for AsyncFunction<F> where
    F: Send
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        (self.function)(frame_data).await
    }
}

#[macro_export]
macro_rules! async_func {
    (async move $body:block) => {
        Box::pin(async move { $body })
    };
}
