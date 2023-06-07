use async_trait::async_trait;

#[async_trait]
pub trait FrameProcessor<F> {
    async fn process(&mut self, frame_data: F) -> Option<F>;
}

pub trait FrameProperties<T> {
    fn set(&mut self, key: &str, value: T);
    fn get(&mut self, key: &str) -> Option<T>;
}

pub trait FrameError<E> {
    fn report_error(&mut self, error: E);
    fn get_error(&mut self) -> Option<E>;
}