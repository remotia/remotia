use std::borrow::Borrow;

use async_trait::async_trait;

#[async_trait]
pub trait FrameProcessor<F> {
    async fn process(&mut self, frame_data: F) -> Option<F>;
}

pub trait FrameProperties<K, V> {
    fn set(&mut self, key: K, value: V);
    fn get(&self, key: &K) -> Option<V>;
}

pub trait BorrowableFrameProperties<K, V> {
    fn push(&mut self, key: K, value: V);
    fn pull(&mut self, key: &K) -> Option<V>;
    fn get_ref(&self, key: &K) -> Option<&V>;
    fn get_mut_ref(&mut self, key: &K) -> Option<&mut V>;
}

pub trait FrameError<E> {
    fn report_error(&mut self, error: E);
    fn get_error(&self) -> Option<E>;
}