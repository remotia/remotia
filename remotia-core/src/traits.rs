use async_trait::async_trait;

#[async_trait]
pub trait FrameProcessor<F> {
    async fn process(&mut self, frame_data: F) -> Option<F>;
}

pub trait FrameProperties<T> {
    fn set(&mut self, key: &str, value: T);
    fn get(&self, key: &str) -> Option<T>;
}

pub trait BorrowableFrameProperties<T> {
    fn push(&mut self, key: &str, value: T);
    fn pull(&mut self, key: &str) -> Option<T>;
    fn get_ref(&self, key: &str) -> Option<&T>;
    fn get_mut_ref(&mut self, key: &str) -> Option<&mut T>;
}

pub trait FrameError<E> {
    fn report_error(&mut self, error: E);
    fn get_error(&self) -> Option<E>;
}