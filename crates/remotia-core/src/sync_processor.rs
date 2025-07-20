use std::marker::PhantomData;

use async_trait::async_trait;

use crate::traits::FrameProcessor;

pub trait SyncFrameProcessor<F: Send + 'static> {
    fn process(&mut self, frame_data: F) -> Option<F>;
}

pub struct SyncProcessorWrapper<F, P>
where
    F: Send + 'static,
    P: SyncFrameProcessor<F>,
{
    processor: P,
    _type: PhantomData<F>,
}

impl<F, P> SyncProcessorWrapper<F, P>
where
    F: Send + 'static,
    P: SyncFrameProcessor<F>,
{
    pub fn new(processor: P) -> Self {
        Self {
            processor,
            _type: PhantomData,
        }
    }
}

#[async_trait]
impl<F, P> FrameProcessor<F> for SyncProcessorWrapper<F, P>
where
    F: Send + 'static,
    P: SyncFrameProcessor<F> + Send,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        self.processor.process(frame_data)
    }
}
