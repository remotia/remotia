use async_trait::async_trait;
use log::debug;

use crate::{
    pipeline::{feeder::PipelineFeeder, Pipeline},
    traits::FrameProcessor,
};

pub struct Sequential<F> {
    processors: Vec<Box<dyn FrameProcessor<F> + Send>>,
}

impl<F> Sequential<F> {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn append<T: 'static + FrameProcessor<F> + Send>(mut self, processor: T) -> Self {
        self.processors.push(Box::new(processor));
        self
    }
}

#[async_trait]
impl<F: Send> FrameProcessor<F> for Sequential<F> {
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let mut result: Option<F> = Some(frame_data);

        for processor in &mut self.processors {
            if result.is_none() {
                break;
            }
            result = processor.process(result.unwrap()).await;
        }

        result
    }
}
