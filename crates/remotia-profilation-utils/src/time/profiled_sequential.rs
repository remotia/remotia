use std::time::Instant;

use async_trait::async_trait;
use remotia_core::{
    processors::containers::sequential::Sequential,
    traits::{FrameProcessor, FrameProperties},
};

pub struct ProfiledSequential<P, F> {
    property_key: P,
    inner_sequential: Sequential<F>,
}

impl<P, F> ProfiledSequential<P, F> {
    pub fn new(property_key: P) -> Self {
        Self {
            property_key,
            inner_sequential: Sequential::new(),
        }
    }

    pub fn append<T>(mut self, processor: T) -> Self
    where
        T: 'static + remotia_core::traits::FrameProcessor<F> + Send,
    {
        self.inner_sequential = self.inner_sequential.append(processor);
        self
    }

    fn inject_time(&self, mut frame_data: F, time: u128) -> F
    where
        F: FrameProperties<P, u128> + Send,
        P: Copy,
    {
        frame_data.set(self.property_key, time);
        frame_data
    }
}

#[async_trait]
impl<P, F> FrameProcessor<F> for ProfiledSequential<P, F>
where
    F: FrameProperties<P, u128> + Send,
    P: Copy + Send,
{
    async fn process(&mut self, frame_data: F) -> Option<F> {
        let start_time = Instant::now();
        let result: Option<F> = self.inner_sequential.process(frame_data).await;
        let time = start_time.elapsed().as_millis();

        log::warn!("Logged time: {}", time);

        result.map(|frame_data| self.inject_time(frame_data, time))
    }
}
