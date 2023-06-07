use async_trait::async_trait;

#[async_trait]
pub trait FrameProcessor<F> {
    async fn process(&mut self, frame_data: F) -> Option<F>;
}

pub trait FrameProperties<T> {
    fn set(&mut self, key: &str, value: T);
    fn get(&mut self, key: &str) -> T;
}