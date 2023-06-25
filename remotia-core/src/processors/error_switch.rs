use std::marker::PhantomData;

use async_trait::async_trait;
use log::debug;

use crate::{
    pipeline::{feeder::PipelineFeeder, Pipeline},
    traits::{FrameError, FrameProcessor},
};

pub struct OnErrorSwitch<F, E> {
    feeder: PipelineFeeder<F>,
    detected_errors: Vec<E>
}

impl<F, E> OnErrorSwitch<F, E> where
    F: std::fmt::Debug + Default + Send + 'static
{
    pub fn new(destination_pipeline: &mut Pipeline<F>) -> Self {
        Self {
            feeder: destination_pipeline.get_feeder(),
            detected_errors: Vec::new()
        }
    }

    pub fn detect(mut self, error: E) -> Self {
        self.detected_errors.push(error);
        self
    }
}

#[async_trait]
impl<F, E> FrameProcessor<F> for OnErrorSwitch<F, E> where 
    E: Send + PartialEq,
    F: FrameError<E> + std::fmt::Debug + Send + 'static,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        if let Some(err) = frame_data.get_error() {
            if self.detected_errors.is_empty() || self.detected_errors.contains(&err) {
                self.feeder.feed(frame_data);
                return None;
            } 
        } 
        
        Some(frame_data)
    }
}
