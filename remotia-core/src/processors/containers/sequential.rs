use async_trait::async_trait;
use log::debug;

use crate::{
    pipeline::ascode::{feeder::AscodePipelineFeeder, AscodePipeline},
    traits::FrameProcessor,
    types::FrameData,
};

pub struct Sequential {
    processors: Vec<Box<dyn FrameProcessor + Send>>,
}

impl Sequential {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn append<T: 'static + FrameProcessor + Send>(mut self, processor: T) -> Self {
        self.processors.push(Box::new(processor));
        self
    }
}

#[async_trait]
impl FrameProcessor for Sequential {
    async fn process(&mut self, frame_data: FrameData) -> Option<FrameData> {
        let mut result: Option<FrameData> = Some(frame_data);

        for processor in &mut self.processors {
            if result.is_none() {
                break;
            }
            result = processor.process(result.unwrap()).await;
        }

        result
    }
}
